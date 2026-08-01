use super::*;
use soroban_sdk::{
    crypto::bls12_381::Bls12381Fr,
    testutils::Address as _,
    Bytes, Env, U256,
};

fn setup(env: &Env) -> (VrfOracleContractClient<'_>, Address, Bls12381Fr) {
    let contract_id = env.register(VrfOracleContract, ());
    let client = VrfOracleContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let sk = Bls12381Fr::from_u256(U256::from_u32(env, 424_242));
    let oracle_pubkey = vrf::pubkey(env, &sk);

    client.initialize(&admin, &oracle_pubkey);
    (client, admin, sk)
}

#[test]
fn full_request_and_fulfill_cycle_succeeds_with_a_valid_proof() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, sk) = setup(&env);
    let requester = Address::generate(&env);
    let seed = Bytes::from_array(&env, &[1, 2, 3, 4]);

    let id = client.request_randomness(&requester, &seed);
    let pending = client.get_request(&id);
    assert_eq!(pending.status, Status::Pending);
    assert!(pending.random.is_none());

    let proof = vrf::sign(&env, &sk, &seed);
    client.fulfill_randomness(&id, &proof);

    let fulfilled = client.get_request(&id);
    assert_eq!(fulfilled.status, Status::Fulfilled);
    assert!(fulfilled.random.is_some());

    // Independently re-derivable: sha256(proof) is exactly the stored value.
    let expected = env.crypto().sha256(&proof.into()).to_bytes();
    assert_eq!(fulfilled.random.unwrap(), expected);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn fulfill_rejects_a_forged_proof() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _sk) = setup(&env);
    let requester = Address::generate(&env);
    let seed = Bytes::from_array(&env, &[1, 2, 3, 4]);
    let id = client.request_randomness(&requester, &seed);

    // Sign with the wrong scalar - a proof that's well-formed but invalid.
    let wrong_sk = Bls12381Fr::from_u256(U256::from_u32(&env, 1));
    let forged_proof = vrf::sign(&env, &wrong_sk, &seed);

    client.fulfill_randomness(&id, &forged_proof);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn fulfill_rejects_a_second_submission_for_the_same_request() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, sk) = setup(&env);
    let requester = Address::generate(&env);
    let seed = Bytes::from_array(&env, &[9, 9, 9]);
    let id = client.request_randomness(&requester, &seed);

    let proof = vrf::sign(&env, &sk, &seed);
    client.fulfill_randomness(&id, &proof);
    client.fulfill_randomness(&id, &proof);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn fulfill_rejects_an_unknown_request_id() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, sk) = setup(&env);
    let seed = Bytes::from_array(&env, &[0]);
    let proof = vrf::sign(&env, &sk, &seed);

    client.fulfill_randomness(&999, &proof);
}

#[test]
fn admin_can_rotate_the_oracle_pubkey() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin, _sk) = setup(&env);
    let new_sk = Bls12381Fr::from_u256(U256::from_u32(&env, 7));
    let new_pubkey = vrf::pubkey(&env, &new_sk);

    client.set_oracle_pubkey(&admin, &new_pubkey);
    assert_eq!(client.get_oracle_pubkey(), new_pubkey);
}

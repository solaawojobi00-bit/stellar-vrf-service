#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    Bytes, BytesN, Env,
};

mod vrf;

#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Pending,
    Fulfilled,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RandomnessRequested {
    #[topic]
    pub request_id: u64,
    pub requester: Address,
    pub seed: Bytes,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct RandomnessFulfilled {
    #[topic]
    pub request_id: u64,
    pub random: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Request {
    pub requester: Address,
    pub seed: Bytes,
    pub status: Status,
    pub random: Option<BytesN<32>>,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    OraclePubkey,
    ReqCount,
    Request(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VrfError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    RequestNotFound = 3,
    AlreadyFulfilled = 4,
    InvalidProof = 5,
}

#[contract]
pub struct VrfOracleContract;

#[contractimpl]
impl VrfOracleContract {
    /// One-time setup. `oracle_pubkey` is the oracle's BLS12-381 G1 public
    /// key (96-byte uncompressed affine encoding), computed off-chain as
    /// `sk * G1_BASE` where `G1_BASE` is this contract's fixed base point
    /// (see `vrf::g1_base`).
    pub fn initialize(env: Env, admin: Address, oracle_pubkey: BytesN<96>) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, VrfError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::OraclePubkey, &oracle_pubkey);
        env.storage().instance().set(&DataKey::ReqCount, &0u64);
    }

    /// Admin-gated oracle key rotation.
    pub fn set_oracle_pubkey(env: Env, admin: Address, new_pubkey: BytesN<96>) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, VrfError::NotInitialized));
        if stored_admin != admin {
            panic_with_error!(&env, VrfError::NotInitialized);
        }
        env.storage()
            .instance()
            .set(&DataKey::OraclePubkey, &new_pubkey);
    }

    /// Requests a random value tied to `seed`. Returns the `request_id`
    /// used to fetch the result once fulfilled. `requester` must authorize
    /// the call.
    pub fn request_randomness(env: Env, requester: Address, seed: Bytes) -> u64 {
        requester.require_auth();
        if !env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, VrfError::NotInitialized);
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ReqCount)
            .unwrap_or(0);
        env.storage().instance().set(&DataKey::ReqCount, &(id + 1));

        let request = Request {
            requester: requester.clone(),
            seed: seed.clone(),
            status: Status::Pending,
            random: None,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Request(id), &request);
        RandomnessRequested {
            request_id: id,
            requester,
            seed,
        }
        .publish(&env);
        id
    }

    /// Submits the oracle's BLS proof for `request_id`. Anyone may relay a
    /// valid proof; security comes from the on-chain pairing check below,
    /// not from who submits the transaction. Aborts if the request does
    /// not exist, was already fulfilled, or the proof fails verification.
    pub fn fulfill_randomness(env: Env, request_id: u64, proof: BytesN<192>) {
        let key = DataKey::Request(request_id);
        let mut request: Request = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, VrfError::RequestNotFound));

        if request.status == Status::Fulfilled {
            panic_with_error!(&env, VrfError::AlreadyFulfilled);
        }

        let oracle_pubkey: BytesN<96> = env
            .storage()
            .instance()
            .get(&DataKey::OraclePubkey)
            .unwrap_or_else(|| panic_with_error!(&env, VrfError::NotInitialized));

        if !vrf::verify(&env, &oracle_pubkey, &request.seed, &proof) {
            panic_with_error!(&env, VrfError::InvalidProof);
        }

        let random = env.crypto().sha256(&proof.clone().into()).to_bytes();
        request.status = Status::Fulfilled;
        request.random = Some(random.clone());
        env.storage().persistent().set(&key, &request);
        RandomnessFulfilled { request_id, random }.publish(&env);
    }

    /// Read-only lookup of a request's current state.
    pub fn get_request(env: Env, request_id: u64) -> Request {
        env.storage()
            .persistent()
            .get(&DataKey::Request(request_id))
            .unwrap_or_else(|| panic_with_error!(&env, VrfError::RequestNotFound))
    }

    /// Returns the currently registered oracle public key.
    pub fn get_oracle_pubkey(env: Env) -> BytesN<96> {
        env.storage()
            .instance()
            .get(&DataKey::OraclePubkey)
            .unwrap_or_else(|| panic_with_error!(&env, VrfError::NotInitialized))
    }
}

#[cfg(test)]
mod test;

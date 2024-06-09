use candid::{types::principal::PrincipalError, Principal};
use serde::{Deserialize, Serialize};
use yral_identity::{msg_builder::Message, Signature};

tonic::include_proto!("yral_metadata");

impl UserMetadata {
    pub fn new(user_canister_id: Principal, user_name: String) -> Self {
        Self {
            user_canister_id: user_canister_id.as_slice().to_vec(),
            user_name,
        }
    }
}

impl TryFrom<UserMetadata> for Message {
    type Error = PrincipalError;
    fn try_from(value: UserMetadata) -> Result<Self, PrincipalError> {
        let canister_id = Principal::try_from_slice(&value.user_canister_id)?;
        Ok(Message::default()
            .method_name("set_user_metadata".into())
            .args((canister_id, value.user_name))
            // unwrap is safe here because (Principal, String) serialization can't fail
            .unwrap())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserMetadataJson {
    pub user_canister_id: Principal,
    pub user_name: String,
}

impl From<UserMetadataJson> for UserMetadata {
    fn from(value: UserMetadataJson) -> Self {
        Self {
            user_canister_id: value.user_canister_id.as_slice().to_vec(),
            user_name: value.user_name,
        }
    }
}

impl TryFrom<UserMetadata> for UserMetadataJson {
    type Error = PrincipalError;
    fn try_from(value: UserMetadata) -> Result<Self, PrincipalError> {
        Ok(Self {
            user_canister_id: Principal::try_from_slice(&value.user_canister_id)?,
            user_name: value.user_name,
        })
    }
}

impl SetUserMetadataReq {
    pub fn new(user_principal: Principal, meta: UserMetadata, signature: Signature) -> Self {
        Self {
            user_metadata: Some(meta),
            signature_json: serde_json::to_string(&signature).unwrap(),
            user_principal_id: user_principal.as_slice().to_vec(),
        }
    }

    pub fn signature(&self) -> Result<Signature, serde_json::Error> {
        serde_json::from_str(&self.signature_json)
    }

    pub fn user_principal(&self) -> Result<Principal, PrincipalError> {
        Principal::try_from_slice(&self.user_principal_id)
    }
}

impl BulkDeleteReq {
    pub fn new(users: impl IntoIterator<Item = Principal>) -> Self {
        Self {
            users: users.into_iter().map(|p| p.as_slice().to_vec()).collect(),
        }
    }
}

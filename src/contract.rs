use crate::error::ContractError;
use crate::msg::{CheckDataResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use babylon_bindings::{BabylonQuerier, BabylonQuery};
use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_storage_plus::Map;
use sha2::{Digest, Sha256};

// Version info for migration info
const CONTRACT_NAME: &str = "storage-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
struct StoredData {
    data: String,
    height: u64,
    timestamp: u64,
    saved_epoch: u64,
}

const STORED_DATA: Map<String, StoredData> = Map::new("data");

fn decode_hex(data: &str) -> Result<Vec<u8>, ContractError> {
    hex::decode(data).map_err(|_| ContractError::HexDecodingError {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<BabylonQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<BabylonQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SaveData { data } => {
            let data_bytes = decode_hex(&data)?;
            let hash = Sha256::digest(data_bytes);
            let hash_string = hex::encode(hash);

            if STORED_DATA.has(deps.storage, hash_string.clone()) {
                return Err(ContractError::DataAlreadyExists {});
            }

            let bq = BabylonQuerier::new(&deps.querier);
            let current_epoch = bq.current_epoch()?;
            // Add BTC timestamp info
            let btc_tip = bq.btc_tip()?;
            let data = StoredData {
                data,
                height: btc_tip.height,
                timestamp: btc_tip.header.time as u64,
                saved_epoch: current_epoch.u64(),
            };

            STORED_DATA.save(deps.storage, hash_string, &data)?;

            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<BabylonQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CheckData { data_hash } => {
            let data = STORED_DATA.load(deps.storage, data_hash)?;
            let bq = BabylonQuerier::new(&deps.querier);
            let latest_finalized_epoch_info_res = bq.latest_finalized_epoch_info();

            // Realistically there can be only one error here i.e there is no finalized epoch
            let latest_finalized_epoch = match latest_finalized_epoch_info_res {
                Ok(epoch_info) => epoch_info.epoch_number,
                Err(_) => 0,
            };

            to_json_binary(&CheckDataResponse {
                height: data.height,
                timestamp: data.timestamp,
                finalized: latest_finalized_epoch >= data.saved_epoch,
                save_epoch: data.saved_epoch,
                latest_finalized_epoch,
            })
        }
    }
}

#[cfg(test)]
mod tests {}

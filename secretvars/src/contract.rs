use cosmwasm_std::{
    entry_point, to_binary, Binary, CanonicalAddr, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};

use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, State};

use secret_toolkit::viewing_key::{ViewingKey, ViewingKeyStore};
use secret_toolkit_crypto::sha_256;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        count: msg.count,
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        allowed_viewers: vec![],
        secret_variables: "".to_string(),
    };

    deps.api
        .debug(format!("Contract was initialized by {}", info.sender).as_str());
    config(deps.storage).save(&state)?;

    let prng_seed_hashed = sha_256(&msg.prng_seed.0);
    ViewingKey::set_seed(deps.storage, &prng_seed_hashed);

    Ok(Response::default())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetViewers { viewers } => try_set_viewers(deps, info, viewers),
        ExecuteMsg::Increment {} => try_increment(deps, env),
        ExecuteMsg::Reset { count } => try_reset(deps, info, count),
        ExecuteMsg::SetSecreteVariables { secret_variables } => {
            try_set_secret_variables(deps, info, secret_variables)
        }
        ExecuteMsg::GenerateViewingKey { entropy } => {
            try_generate_viewing_key(deps, info, env, entropy)
        }
    }
}

pub fn try_generate_viewing_key(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    entropy: String,
) -> StdResult<Response> {
    let sender_address_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    let state = config_read(deps.storage).load()?;

    if !state.allowed_viewers.contains(&sender_address_raw) {
        return Err(StdError::generic_err(
            "Only allowed viewers can generate viewing keys",
        ));
    }

    let key = ViewingKey::create(
        deps.storage,
        &info,
        &env,
        info.sender.as_str(),
        entropy.as_ref(),
    );

    Ok(Response::new().set_data(to_binary(key.as_bytes())?))
}

pub fn try_set_secret_variables(
    deps: DepsMut,
    info: MessageInfo,
    secret_variables: String,
) -> StdResult<Response> {
    let sender_address_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    let state = config_read(deps.storage).load()?;

    if sender_address_raw != state.owner {
        return Err(StdError::generic_err(
            "Only the owner can set secret variables",
        ));
    }

    config(deps.storage).update(|mut state| -> Result<_, StdError> {
        state.secret_variables = secret_variables;
        Ok(state)
    })?;

    deps.api.debug("secret variables set successfully");
    Ok(Response::default())
}

pub fn try_set_viewers(
    deps: DepsMut,
    info: MessageInfo,
    viewers: Vec<String>,
) -> StdResult<Response> {
    let sender_address_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    let temp_allowed_viewers: Vec<CanonicalAddr> = viewers
        .iter()
        .map(|v| deps.api.addr_canonicalize(v.as_str()).unwrap())
        .collect();

    config(deps.storage).update(|mut state| -> Result<_, StdError> {
        if sender_address_raw != state.owner {
            return Err(StdError::generic_err("Only the owner can set viewers"));
        }

        state.allowed_viewers = temp_allowed_viewers;

        Ok(state)
    })?;

    deps.api.debug("viewers set successfully");
    Ok(Response::default())
}

pub fn try_increment(deps: DepsMut, _env: Env) -> StdResult<Response> {
    config(deps.storage).update(|mut state| -> Result<_, StdError> {
        state.count += 1;
        Ok(state)
    })?;

    deps.api.debug("count incremented successfully");
    Ok(Response::default())
}

pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> StdResult<Response> {
    let sender_address_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
    config(deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
            return Err(StdError::generic_err("Only the owner can reset count"));
        }
        state.count = count;
        Ok(state)
    })?;

    deps.api.debug("count reset successfully");
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::GetSecretVariables {
            viewing_key,
            account,
        } => to_binary(&query_secret_variables(deps, viewing_key, account)?),
    }
}

fn query_secret_variables(deps: Deps, viewing_key: String, account: String) -> StdResult<String> {
    let state = config_read(deps.storage).load()?;
    let result = ViewingKey::check(deps.storage, account.as_ref(), viewing_key.as_ref());

    if !result.is_ok() {
        return Err(StdError::generic_err(
            "Only allowed viewers can query secret variables",
        ));
    }

    Ok(state.secret_variables)
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let state = config_read(deps.storage).load()?;
    Ok(CountResponse { count: state.count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::*;
    use cosmwasm_std::{from_binary, Coin, StdError, Uint128};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let info = mock_info(
            "creator",
            &[Coin {
                denom: "earth".to_string(),
                amount: Uint128::new(1000),
            }],
        );
        let init_msg = InstantiateMsg {
            count: 17,
            prng_seed: b"prng_seed".to_vec().into(),
        };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "token".to_string(),
            amount: Uint128::new(2),
        }]);
        let info = mock_info(
            "creator",
            &[Coin {
                denom: "token".to_string(),
                amount: Uint128::new(2),
            }],
        );
        let init_msg = InstantiateMsg {
            count: 17,
            prng_seed: b"prng_seed".to_vec().into(),
        };

        let _res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // anyone can increment
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: "token".to_string(),
                amount: Uint128::new(2),
            }],
        );

        let exec_msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "token".to_string(),
            amount: Uint128::new(2),
        }]);
        let info = mock_info(
            "creator",
            &[Coin {
                denom: "token".to_string(),
                amount: Uint128::new(2),
            }],
        );
        let init_msg = InstantiateMsg {
            count: 17,
            prng_seed: b"prng_seed".to_vec().into(),
        };

        let _res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // not anyone can reset
        let info = mock_info(
            "anyone",
            &[Coin {
                denom: "token".to_string(),
                amount: Uint128::new(2),
            }],
        );
        let exec_msg = ExecuteMsg::Reset { count: 5 };

        let res = execute(deps.as_mut(), mock_env(), info, exec_msg);

        match res {
            Err(StdError::GenericErr { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let info = mock_info(
            "creator",
            &[Coin {
                denom: "token".to_string(),
                amount: Uint128::new(2),
            }],
        );
        let exec_msg = ExecuteMsg::Reset { count: 5 };

        let _res = execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}

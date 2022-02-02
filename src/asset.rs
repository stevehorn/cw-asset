use std::fmt;

use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::asset_info::{AssetInfo, AssetInfoBase};

/// Represents a fungible asset with a known amount
///
/// Each asset instance contains two values: [`info`], which specifies the asset's type (CW20 or
/// native), and its [`amount`], which specifies the asset's amount
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetBase<T> {
    /// Specifies the asset's type (CW20 or native)
    pub info: AssetInfoBase<T>,
    /// Specifies the asset's amount
    pub amount: Uint128,
}

impl<T> AssetBase<T> {
    /// Create a new **asset** instance based on given asset info and amount
    ///
    /// To create an unchecked instance, the [`info`] parameter may be either checked or unchecked;
    /// to create a checked instance, the [`info`] paramter must also be checked.
    ///
    /// ```rust
    /// use cosmwasm_std::Addr;
    /// use cw_asset::{Asset, AssetInfo};
    ///
    /// let info1 = AssetInfo::cw20(Addr::unchecked("token_addr"));
    /// let asset1 = Asset::new(info1, 12345u128);
    ///
    /// let info2 = AssetInfo::native("uusd");
    /// let asset2 = Asset::new(info2, 67890u128);
    /// ```
    pub fn new<A: Into<AssetInfoBase<T>>, B: Into<Uint128>>(info: A, amount: B) -> Self {
        Self {
            info: info.into(),
            amount: amount.into(),
        }
    }

    /// Create a new **asset** instance representing a CW20 token of given contract address and amount
    ///
    /// To create an unchecked instance, provide the contract address in any of the following types:
    /// [`cosmwasm_std::Addr`], [`String`], or [`&str`]; to create a checked instance, the address
    /// must of type [`cosmwasm_std::Addr`].
    ///
    /// ```rust
    /// use cosmwasm_std::Addr;
    /// use cw_asset::Asset;
    ///
    /// let asset = Asset::cw20(Addr::unchecked("token_addr"), 12345u128);
    /// ```
    pub fn cw20<A: Into<T>, B: Into<Uint128>>(contract_addr: A, amount: B) -> Self {
        Self {
            info: AssetInfoBase::cw20(contract_addr),
            amount: amount.into(),
        }
    }

    /// Create a new **asset** instance representing a native coin of given denom and amount
    ///
    /// ```rust
    /// use cw_asset::Asset;
    ///
    /// let asset = Asset::native("uusd", 12345u128);
    /// ```
    pub fn native<A: Into<String>, B: Into<Uint128>>(denom: A, amount: B) -> Self {
        Self {
            info: AssetInfoBase::native(denom),
            amount: amount.into(),
        }
    }
}

// Represents an **asset** instance that may contain unverified data; to be used in messages
pub type AssetUnchecked = AssetBase<String>;
// Represents an **asset** instance containing only verified data; to be saved in contract storage
pub type Asset = AssetBase<Addr>;

impl From<Asset> for AssetUnchecked {
    fn from(asset: Asset) -> Self {
        AssetUnchecked {
            info: asset.info.into(),
            amount: asset.amount,
        }
    }
}

impl AssetUnchecked {
    /// Validate data contained in an _unchecked_ **asset** instnace, return a new _checked_
    /// **asset** instance
    ///
    /// ```rust
    /// use cosmwasm_std::{Addr, Api};
    /// use cw_asset::{Asset, AssetUnchecked};
    ///
    /// fn validate_asset(api: &dyn Api, asset_unchecked: &AssetUnchecked) {
    ///     match asset_unchecked.check(api) {
    ///         Ok(asset) => println!("asset is valid: {}", asset.to_string()),
    ///         Err(err) => println!("asset is invalid! reason: {}", err)
    ///     }
    /// }
    /// ```
    pub fn check(&self, api: &dyn Api) -> StdResult<Asset> {
        Ok(Asset {
            info: self.info.check(api)?,
            amount: self.amount,
        })
    }

    /// Similar to `check`, but in case `self` is a native token, also verifies its denom is included
    /// in a given whitelist
    pub fn check_whitelist(&self, api: &dyn Api, whitelist: &[&str]) -> StdResult<Asset> {
        Ok(Asset {
            info: self.info.check_whitelist(api, whitelist)?,
            amount: self.amount,
        })
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.info, self.amount)
    }
}

impl From<Coin> for Asset {
    fn from(coin: Coin) -> Self {
        Self {
            info: AssetInfo::Native(coin.denom),
            amount: coin.amount,
        }
    }
}

impl From<&Coin> for Asset {
    fn from(coin: &Coin) -> Self {
        coin.clone().into()
    }
}

impl Asset {
    /// Generate a message that sends a CW20 token to the specified recipient with a binary payload
    ///
    /// NOTE: Only works for CW20 tokens. Returns error if invoked on an [`Asset`] instance
    /// representing a native coin, as native coins do not have an equivalent method mplemented.  
    ///
    /// ```rust
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// enum MockReceiveMsg {
    ///     MockCommand {}
    /// }
    ///
    /// use cosmwasm_std::{to_binary, Addr, Response, StdResult};
    /// use cw_asset::Asset;
    ///
    /// fn send_asset(asset: &Asset, contract_addr: &Addr, msg: &MockReceiveMsg) -> StdResult<Response> {
    ///     let msg = asset.send_msg(contract_addr, to_binary(msg)?)?;
    ///
    ///     Ok(Response::new()
    ///         .add_message(msg)
    ///         .add_attribute("asset_sent", asset.to_string()))
    /// }
    /// ```
    pub fn send_msg<A: Into<String>>(&self, to: A, msg: Binary) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: to.into(),
                    amount: self.amount,
                    msg,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(_) => {
                Err(StdError::generic_err("native coins do not have `send` method"))
            }
        }
    }

    /// Generate a message that transfers the asset from the sender to to a specified account
    ///
    /// ```rust
    /// use cosmwasm_std::{Addr, Response, StdResult};
    /// use cw_asset::Asset;
    ///
    /// fn transfer_asset(asset: &Asset, recipient_addr: &Addr) -> StdResult<Response> {
    ///     let msg = asset.transfer_msg(recipient_addr)?;
    ///
    ///     Ok(Response::new()
    ///         .add_message(msg)
    ///         .add_attribute("asset_sent", asset.to_string()))
    /// }
    /// ```
    pub fn transfer_msg<A: Into<String>>(&self, to: A) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to.into(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(denom) => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: to.into(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: self.amount,
                }],
            })),
        }
    }

    /// Generate a message that draws the asset from the account specified by [`from`] to the one
    /// specified by [`to`]
    ///
    /// NOTE: Only works for CW20 tokens. Returns error if invoked on an [`Asset`] instance
    /// representing a native coin, as native coins do not have an equivalent method mplemented.  
    ///
    /// ```rust
    /// use cosmwasm_std::{Addr, Response, StdResult};
    /// use cw_asset::Asset;
    ///
    /// fn draw_asset(asset: &Asset, user_addr: &Addr, contract_addr: &Addr) -> StdResult<Response> {
    ///     let msg = asset.transfer_from_msg(user_addr, contract_addr)?;
    ///
    ///     Ok(Response::new()
    ///         .add_message(msg)
    ///         .add_attribute("asset_drawn", asset.to_string()))
    /// }
    /// ```
    pub fn transfer_from_msg<A: Into<String>, B: Into<String>>(
        &self,
        from: A,
        to: B,
    ) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Cw20(contract_addr) => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: from.into(),
                    recipient: to.into(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::Native(_) => {
                Err(StdError::generic_err("native coins do not have `transfer_from` method"))
            }
        }
    }
}

#[cfg(feature = "legacy")]
impl From<Asset> for astroport::asset::Asset {
    fn from(asset: Asset) -> Self {
        Self {
            info: asset.info.into(),
            amount: asset.amount,
        }
    }
}

#[cfg(feature = "legacy")]
impl From<&Asset> for astroport::asset::Asset {
    fn from(asset: &Asset) -> Self {
        asset.clone().into()
    }
}

#[cfg(feature = "legacy")]
impl From<astroport::asset::Asset> for Asset {
    fn from(legacy_asset: astroport::asset::Asset) -> Self {
        Self {
            info: legacy_asset.info.into(),
            amount: legacy_asset.amount,
        }
    }
}

#[cfg(feature = "legacy")]
impl From<&astroport::asset::Asset> for Asset {
    fn from(legacy_asset: &astroport::asset::Asset) -> Self {
        legacy_asset.clone().into()
    }
}

#[cfg(feature = "legacy")]
impl std::cmp::PartialEq<Asset> for astroport::asset::Asset {
    fn eq(&self, other: &Asset) -> bool {
        self.info == other.info && self.amount == other.amount
    }
}

#[cfg(feature = "legacy")]
impl std::cmp::PartialEq<astroport::asset::Asset> for Asset {
    fn eq(&self, other: &astroport::asset::Asset) -> bool {
        other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AssetInfoUnchecked;
    use cosmwasm_std::testing::MockApi;

    #[derive(Serialize)]
    enum MockExecuteMsg {
        MockCommand {},
    }

    #[test]
    fn creating_instances() {
        let info = AssetInfo::native("uusd");
        let asset = Asset::new(info, 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        );

        let asset = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Cw20(Addr::unchecked("mock_token")),
                amount: Uint128::new(123456u128)
            }
        );

        let asset = Asset::native("uusd", 123456u128);
        assert_eq!(
            asset,
            Asset {
                info: AssetInfo::Native(String::from("uusd")),
                amount: Uint128::new(123456u128)
            }
        )
    }

    #[test]
    fn comparing() {
        let uluna1 = Asset::native("uluna", 69u128);
        let uluna2 = Asset::native("uluna", 420u128);
        let uusd = Asset::native("uusd", 69u128);
        let astro = Asset::cw20(Addr::unchecked("astro_token"), 69u128);

        assert_eq!(uluna1 == uluna2, false);
        assert_eq!(uluna1 == uusd, false);
        assert_eq!(astro == astro.clone(), true);
    }

    #[test]
    fn displaying() {
        let asset = Asset::native("uusd", 69420u128);
        assert_eq!(asset.to_string(), String::from("native:uusd:69420"));

        let asset = Asset::cw20(Addr::unchecked("mock_token"), 88888u128);
        assert_eq!(asset.to_string(), String::from("cw20:mock_token:88888"));
    }

    #[test]
    fn checking() {
        let api = MockApi::default();

        let checked = Asset::cw20(Addr::unchecked("mock_token"), 12345u128);
        let unchecked: AssetUnchecked = checked.clone().into();
        assert_eq!(unchecked.check(&api).unwrap(), checked);

        let checked = Asset::native("uusd", 12345u128);
        let unchecked: AssetUnchecked = checked.clone().into();
        assert_eq!(unchecked.check_whitelist(&api, &["uusd", "uluna", "uosmo"]).unwrap(), checked);

        let unchecked = AssetUnchecked::new(AssetInfoUnchecked::native("uatom"), 12345u128);
        assert_eq!(
            unchecked.check_whitelist(&api, &["uusd", "uluna", "uosmo"]),
            Err(StdError::generic_err("invalid denom uatom; must be uusd|uluna|uosmo")),
        );
    }

    #[test]
    fn creating_messages() {
        let token = Asset::cw20(Addr::unchecked("mock_token"), 123456u128);
        let coin = Asset::native("uusd", 123456u128);

        let bin_msg = to_binary(&MockExecuteMsg::MockCommand {}).unwrap();
        let msg = token.send_msg("mock_contract", bin_msg.clone()).unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: String::from("mock_contract"),
                    amount: Uint128::new(123456),
                    msg: to_binary(&MockExecuteMsg::MockCommand {}).unwrap()
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let err = coin.send_msg("mock_contract", bin_msg);
        assert_eq!(err, Err(StdError::generic_err("native coins do not have `send` method")));

        let msg = token.transfer_msg("alice").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
                    amount: Uint128::new(123456)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let msg = coin.transfer_msg("alice").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: String::from("alice"),
                amount: vec![Coin::new(123456, "uusd")]
            })
        );

        let msg = token.transfer_from_msg("bob", "charlie").unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("mock_token"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: String::from("bob"),
                    recipient: String::from("charlie"),
                    amount: Uint128::new(123456)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let err = coin.transfer_from_msg("bob", "charlie");
        assert_eq!(
            err,
            Err(StdError::generic_err("native coins do not have `transfer_from` method"))
        );
    }
}

#[cfg(all(test, feature = "legacy"))]
mod tests_legacy {
    use super::*;

    fn legacy_uusd() -> astroport::asset::AssetInfo {
        astroport::asset::AssetInfo::NativeToken {
            denom: String::from("uusd"),
        }
    }

    fn legacy_uluna() -> astroport::asset::AssetInfo {
        astroport::asset::AssetInfo::NativeToken {
            denom: String::from("uluna"),
        }
    }

    #[test]
    fn casting_legacy() {
        let legacy_asset = astroport::asset::Asset {
            info: legacy_uusd(),
            amount: Uint128::new(69420),
        };

        let asset = Asset::native("uusd", 69420u128);

        assert_eq!(asset, Asset::from(&legacy_asset));
        assert_eq!(asset, Asset::from(legacy_asset.clone()));
        assert_eq!(legacy_asset, astroport::asset::Asset::from(&asset));
        assert_eq!(legacy_asset, astroport::asset::Asset::from(asset));
    }

    #[test]
    fn comparing() {
        let legacy_asset_1 = astroport::asset::Asset {
            info: legacy_uusd(),
            amount: Uint128::new(69420),
        };
        let legacy_asset_2 = astroport::asset::Asset {
            info: legacy_uusd(),
            amount: Uint128::new(88888),
        };
        let legacy_asset_3 = astroport::asset::Asset {
            info: legacy_uluna(),
            amount: Uint128::new(69420),
        };

        let asset = Asset::native("uusd", 69420u128);

        assert_eq!(legacy_asset_1 == asset, true);
        assert_eq!(legacy_asset_2 == asset, false);
        assert_eq!(legacy_asset_3 == asset, false);
    }
}

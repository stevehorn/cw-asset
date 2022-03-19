#![forbid(unsafe_code)]
//! A unified representation of various types of Cosmos fungible assets, and helper functions for  
//! interacting with them
//!
//! ## Basic usage
//!
//! The following code generates messages the sends some SDK coins and CW20 tokens to a recipient:
//!
//! ```rust
//! use cosmwasm_std::{Api, Response, StdResult};
//! use cw_asset::Asset;
//!
//! fn transfer_two_assets(api: &dyn Api) -> StdResult<Response> {
//!     let asset1 = Asset::native("uusd", 12345u128);
//!     let msg1 = asset1.transfer_msg("recipient_addr")?;
//!
//!     let asset2 = Asset::cw20(api.addr_validate("token_addr")?, 67890u128);
//!     let msg2 = asset1.transfer_msg("recipient_addr")?;
//!
//!     Ok(Response::new()
//!         .add_message(msg1)
//!         .add_message(msg2)
//!         .add_attribute("asset_sent", asset1.to_string())
//!         .add_attribute("asset_sent", asset2.to_string()))
//! }
//! ```
//!
//! ## Asset list
//!
//! An [`AssetList`] struct is also provided for dealing with multiple assets at the same time:
//!
//! ```rust
//! use cosmwasm_std::{Api, Response, StdResult};
//! use cw_asset::{Asset, AssetList};
//!
//! fn transfer_multiple_assets(api: &dyn Api) -> StdResult<Response> {
//!     let assets = AssetList::from(vec![
//!         Asset::native("uusd", 12345u128),
//!         Asset::cw20(api.addr_validate("token_addr")?, 67890u128)
//!     ]);
//!
//!     let msgs = assets.transfer_msgs(api.addr_validate("recipient_addr")?)?;
//!
//!     Ok(Response::new()
//!         .add_messages(msgs)
//!         .add_attribute("assets_sent", assets.to_string()))
//! }
//! ```
//!
//! ## Use in messages
//!
//! [`Asset`] and [`AssetList`] each comes with an _unchecked_ counterpart which contains unverified
//! addresses and/or denoms, and implements traits that allow them to be serialized into JSON, so 
//! that they can be directly used in Cosmos messages:
//!
//! ```rust
//! use serde::{Serialize, Deserialize};
//! use schemars::JsonSchema;
//!
//! use cw_asset::AssetUnchecked;
//!
//! #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
//! pub enum ExecuteMsg {
//!     Deposit {
//!         asset: AssetUnchecked,
//!     }
//! }
//! ```
//!
//! Although [`Asset`] and [`AssetList`] _also_ implement the related traits, hence can also be used
//! in messages, it is not recommended to do so; it is a good security practice to never trust
//! addresses passed in by messages to be valid. Instead, also validate them yourselves:
//!
//! ```rust
//! use cosmwasm_std::{Api, StdResult};
//! use cw_asset::{Asset, AssetUnchecked};
//! 
//! const ACCEPTED_DENOMS: &[&str] = &["uatom", "uosmo", "uluna"];
//!
//! fn validate_deposit(api: &dyn Api, asset_unchecked: AssetUnchecked) -> StdResult<()> {
//!     let asset: Asset = asset_unchecked.check(api, Some(ACCEPTED_DENOMS))?;
//!     Ok(())
//! }
//! ```
//!
//! ## Legacy feature
//!
//! Some projects may still use their legacy asset types. Enabling the `astroport` or `mars` 
//! features imports methods that cast `Asset` into these legacy types used by these projects.
//!
//! In `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! cw-asset = { version = "^1.0", features = ["astroport"] }
//! ```
//!
//! In the contract:
//!
//! ```rust
//! use astroport::asset::Asset as AstroportAsset;
//! use cw_asset::Asset;
//!
//! let asset = Asset::native("uusd", 12345u128);
//! let legacy_asset: AstroportAsset = asset.into();
//! ```
mod asset;
mod asset_info;
mod asset_list;

pub use asset::*;
pub use asset_info::*;
pub use asset_list::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod testing;

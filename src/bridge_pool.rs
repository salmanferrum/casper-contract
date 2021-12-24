
// import "@openzeppelin/contracts/utils/cryptography/draft-EIP712.sol";
// import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
// import "@openzeppelin/contracts/access/Ownable.sol";
// import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
// import "../common/SafeAmount.sol";
// import "../token/TaxDistributor.sol";

// extern crate bigint;
// use bigint::U256;
use std::collections::HashMap;
extern crate alloc;
use alloc::string::String;
use secp256k1::Secp256k1;
use crate::crypto_utils::EcdsaSig;
use crate::crypto_utils::ecdsa_recover;

use casper_contract::{contract_api::runtime, unwrap_or_revert::UnwrapOrRevert};
use casper_erc20::{
    constants::{
        ADDRESS_RUNTIME_ARG_NAME, AMOUNT_RUNTIME_ARG_NAME, DECIMALS_RUNTIME_ARG_NAME,
        NAME_RUNTIME_ARG_NAME, OWNER_RUNTIME_ARG_NAME, RECIPIENT_RUNTIME_ARG_NAME,
        SPENDER_RUNTIME_ARG_NAME, SYMBOL_RUNTIME_ARG_NAME, TOTAL_SUPPLY_RUNTIME_ARG_NAME,
    },
    Address, ERC20,
};
use casper_types::{CLValue, U256};


// I don't have access to this interface/trait
#[allow(dead_code)]

// you can change the number of bytes or completely the type
type SignerAddress = [u8; 32];
type Bytes = [u8; 32];
type Bytes8 = [u8];
type Liquidity = HashMap<Address, HashMap<Address, U256>>;
const ZERO_ADDRESS: [u8; 32] = [0u8; 32]; // address(0)
const ZERO: U256 = U256::from(0);
const TEN_THOUSAND: U256 = U256::from(10000);
const NAME: &'static str = "FERRUM_TOKEN_BRIDGE_POOL";
const VERSION: &'static str = "000.001";

#[derive(Debug)]
pub struct BridgePool {
    signer: SignerAddress,
    usedHashes: HashMap<Bytes, bool>,
    fees: HashMap<Address, U256>,
    feeDistributor: Address,
    liquidities: Liquidity
}


// constructor () EIP712(NAME, VERSION) { }

impl BridgePool {
    pub fn set_signer(&mut self, signer: Address)-> Result<(), String>{
        if signer == ZERO_ADDRESS {
            return Err("Bad Signer".into());
        }

        self.signer = signer;
        
        Ok(())
    }

    pub fn set_fee(&mut self, token: Address, fee_10000: U256) -> Result<(), String>{
        if token == ZERO_ADDRESS {
            return Err("Bad Token".into());
        }

        self.fees.insert(token, fee_10000).expect("Insertion failed");
        
        Ok(())
    }

    pub fn set_fee_distributor(&mut self, fee_distributor: Address) {
        self.feeDistributor = fee_distributor;
    }

    pub fn swap(
        &mut self,
        sender: Address,
        token: Address, 
        amount: U256, 
        target_network: U256, 
        target_token: Address,
    ) -> U256 { 
        return self.swap_helper(sender, token, amount, target_network, target_token, ZERO_ADDRESS);
    }

    pub fn swap_to_address(
        &mut self,
        sender: Address,
        token: Address,
        amount: U256, 
        target_network: U256, 
        target_token: Address,
        target_address: Address,
    ) -> Result<U256, String> { 
        if target_address == ZERO_ADDRESS {
            return Err("BridgePool: targetAddress is required".into());
        }
        Ok(self.swap_helper(sender, token, amount, target_network, target_token, target_address))
    }

    fn swap_helper(
        &mut self,
        from: Address, 
        token: Address, 
        amount: U256, 
        target_network: U256, 
        target_token: Address,
        target_address: Address
    ) -> U256 {
    let fees = self.fees.get(&token).unwrap();
    let zero_0: i32 = 0;
    let ten_000: i32 = 10000;
    let zero: U256 = U256::from(zero_0);
    let ten_thousand = U256::from(ten_000);

    let actual_amount: U256 = amount;
    let mut fee;
    let fee_distributor: Address = self.feeDistributor;
    
    if fee_distributor == ZERO_ADDRESS {
        fee = amount * *fees / ten_thousand;

        actual_amount = amount - fee;

        if fee != zero {
            // IERC20(token).transferFrom(from, _feeDistributor, fee);
        }
    }
    // IERC20(token).transferFrom(from, address(this), actualAmount);
    // emit BridgeSwap(from, token, targetNetwork, targetToken, targetAddress, actualAmount, fee);
    return actual_amount;
}

pub fn add_liquidity(&mut self, sender: Address, token: Address, amount: U256) -> Result<(), String>{
    

    if amount == ZERO && token == ZERO_ADDRESS {
        return Err("Amount must be positive & bad token provided".into());
    }
    // amount = SafeAmount.safeTransferFrom(token, msg.sender, address(this), amount);

    if let Some(inner_hash_map) =  self.liquidities.get_mut(&token) {
        let inner_hash_value = inner_hash_map.get(&sender).unwrap_or(&ZERO);
        let result = *inner_hash_value + amount;
        let _ = inner_hash_map.insert(sender, result);
    }

    Ok(())

    // emit BridgeLiquidityAdded(sender, token, amount);    
} 

pub fn remove_liquidity_ifpossible(
    &mut self, 
    sender: Address,
    token: Address,
    amount: U256
) -> Result<U256, String> {
    if amount == ZERO && token == ZERO_ADDRESS {
        return Err("Amount must be positive and bad token provided".into());
    }

    let liq = self.liquidities.get(&token).unwrap().get(&sender).unwrap();
    
    if liq <= &amount {
        return Err("Not enough Liquidity".into());
    }
    
    let balance = ERC20::default().balance_of(sender);
    // let balance: U256 = ERC20(&token).balanceOf(sender);

    // uint256 actualLiq = balance > amount ? amount : balance;
    let actual_liq = if balance > amount { amount } else { balance };

    // liquidities[token][msg.sender] = liquidities[token][msg.sender].sub(actualLiq);

    if let Some(inner_hash_map) =  self.liquidities.get_mut(&token) {
        let inner_hash_value = inner_hash_map.get(&sender).unwrap_or(&ZERO);
        let result = *inner_hash_value - actual_liq;
        let _ = inner_hash_map.insert(sender, result);
    }

    if actual_liq != ZERO {
        // IERC20(token).safeTransfer(msg.sender, actualLiq);
        // emit BridgeLiquidityRemoved(sender, token, amount);
    }
    Ok(actual_liq)
}

pub fn liquidity(
    &mut self, 
    token: Address,
    liquidity_adder: Address
)-> &U256 {
    
    if self.liquidities.get(&token).unwrap().get(&liquidity_adder).unwrap() == &ZERO
    {
         return &ZERO;
    }
    return self.liquidities.get(&token).unwrap().get(&liquidity_adder).unwrap();
}

pub fn withdraw_signed(
    &mut self,
    token: Address,
    payee: Address,
    amount: U256,
    salt: Bytes,
    signature: &[u8]
) -> Result<Bytes, String> {
    let digest: Bytes;


    // digest = _hashTypedDataV4(keccak256(abi.encode(
    //   keccak256("WithdrawSigned(address token, address payee,uint256 amount,bytes32 salt)"),
    //      token, payee, amount, salt)));

    if !self.usedHashes.get(&digest).unwrap() { 
        return Err("Message already used".into());
    }
    let sig = EcdsaSig::from(signature).unwrap();
    let _signer = ecdsa_recover(&digest, &sig).unwrap();  // if we can use ECDSA Openzepplin interface
    
    if _signer != self.signer {
        return Err("Bridge Pool: Invalid Signer".into());
    }

    self.usedHashes.insert(digest, true);
    // IERC20(token).safeTransfer(payee, amount);
    // emit TransferBySignature(digest, _signer, payee, token, amount);

   Ok(digest)
}

}















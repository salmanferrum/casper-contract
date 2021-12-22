use bigint::U256;
use std::collections::HashMap;

// I don't have access to this interface/trait
#[allow(dead_code)]

pub trait IRewardDistributor {
    fn roll_and_get_distribution_address(&self, address: Address) -> Address;

    fn update_rewards(&self, address: Address) -> bool;
}

// you can change the number of bytes or completely the type
type Address = [u8; 32];

const ZERO_ADDRESS: [u8; 32] = [0u8; 32];  // address(0)
const DEV_FEE_MIN_AMOUNT: u8 = 100;

// traut is an interface
trait IERC20Burnable {
    fn burn(amount: U256);
}

trait ITaxDistributor {
    fn distribute_tax(token: Address) -> bool;
}

#[derive(Debug)]
struct Distribution {
    stake: u8,
    burn: u8,
    future: u8,
    dev: u8,
}

pub struct TaxDistributor<D> {
    distribution: HashMap<Address, Distribution>,
    reward_distributor: HashMap<Address, D>,
    dev_address: HashMap<Address, Address>,
    future_address: HashMap<Address, Address>,
    global_dev_address: Address,
    global_dev_fee_per_100: U256,
}

impl<D> TaxDistributor<D>
where
    D: IRewardDistributor,
{
    // careful with these defaults, 
    // I added this just for testing purposes

    //hypothetical constructor
    pub fn default() -> Self {
        Self {
            distribution: Default::default(),
            reward_distributor: Default::default(),
            dev_address: Default::default(),
            future_address: Default::default(),
            global_dev_address: Default::default(),
            global_dev_fee_per_100: U256::from(0),
        }
    }
// Another declaration for default trait
    // pub fn default() -> Self {
    //     Self {
    //         global_dev_fee_per_100: U256::from(0), 
    //         ..Default::default()
    //     }
    // }

    pub fn set_reward_distributor(
        &mut self,
        sender: Address, // included sender address since msg.address is only specific to Solidity
        token: Address,
        rwrd_distributor: D,
        //Result is a type that represents 
        //either success (Ok) or failure (Err).
    ) -> Result<(), String> {
        if token == ZERO_ADDRESS {
            return Err("TaxDistributor: Bad token".into());
        }
        //address someAddress = rd.rollAndGetDistributionAddress(msg.sender);
        let some_address = rwrd_distributor.roll_and_get_distribution_address(sender);

        if some_address == ZERO_ADDRESS {
            return Err("TaxDistributor: Bad reward distributor".into());
        }
        // rewardDistributor[token] = rd
        self.reward_distributor.insert(token, rwrd_distributor);

        Ok(())  // return true
    }

    pub fn set_dev_address(&mut self, token: Address, dev_addr: Address) -> Result<(), String> {
        if token == ZERO_ADDRESS {
            return Err("TaxDistributor: Bad token".into());
        }
        // devAddress[token] = _devAddress; // Allow 0
        self.dev_address.insert(token, dev_addr);

        Ok(())
    }

    pub fn set_global_dev_address(
        &mut self,
        dev_add: Address,
        dev_fee_per_100: U256,
    ) -> Result<(), String> {
        if dev_fee_per_100 < U256::from(DEV_FEE_MIN_AMOUNT) {
            return Err("TaxDistributor: Invalid dev_fee_per_100".into());
        }

        self.global_dev_address = dev_add;
        self.global_dev_fee_per_100 = dev_fee_per_100;

        Ok(())
    }

    pub fn set_future_address(&mut self, token: Address, fut_addr: Address) -> Result<(), String> {
        if token == ZERO_ADDRESS {
            return Err("TaxDistributor: Bad token".into());
        }

        self.future_address.insert(token, fut_addr);
        Ok(())
    }

    pub fn set_default_distribution(
        &mut self,
        token: Address,
        stake: u8,
        burn: u8,
        dev: u8,
        future: u8,
    ) -> Result<(), String> {
        if token == ZERO_ADDRESS {
            return Err("TaxDistributor: Bad token".into());
        }

        if stake + burn + dev + future == 100 {
            return Err("StakeDevBurnTaxable: taxes must add to 100".into());
        }

        self.distribution.insert(
            token,
            Distribution {
                stake,
                burn,
                dev,
                future,
            },
        );

        Ok(())
    }

    // included sender
    fn distribute_tax(&self, sender: Address, token: Address, amount: U256) -> Result<(), String> {
        let dist = self.distribution.get(&token).unwrap();
        let zero = U256::from(0);
        let hundred = U256::from(100);
        let global_dev_fee_per_100 = self.global_dev_fee_per_100;
        let mut remaining = amount;

        if global_dev_fee_per_100 != zero {
            let global_dev_amount = amount * global_dev_fee_per_100 / hundred;

            if global_dev_amount != zero {
                // IERC20(token).transfer(globalDevAddress, global_dev_amount);
                remaining = remaining - global_dev_amount;
            }
        }

        if dist.burn != 0 {
            let burn_amount = amount * U256::from(dist.burn) / hundred;

            if burn_amount != zero {
                //IERC20Burnable(token).burn(burn_amount);
                remaining = remaining - burn_amount;
            }
        }

        if dist.dev != 0 {
            let dev_amount = amount * U256::from(dist.dev) / hundred;
            if dev_amount != zero {
                //IERC20(token).transfer(devAddress[token], dev_amount);
                remaining = remaining - dev_amount
            }
        }

        if dist.future != 0 {
            let future_amunt = amount * U256::from(dist.future) / hundred;
            if future_amunt != zero {
                //IERC20(token).transfer(futureAddress[token], future_amunt);
                remaining = remaining - future_amunt;
            }
        }

        if dist.stake != 0 {
            let _stake_amount = remaining;
            let reward_distributor = self.reward_distributor.get(&token).unwrap();
            let stake_address = reward_distributor.roll_and_get_distribution_address(sender);
            if stake_address != ZERO_ADDRESS {
                //IERC20(token).transfer(stakeAddress, stake_amount);
                if !reward_distributor.update_rewards(stake_address) {
                    return Err("StakeDevBurnTaxable: Error staking rewards".into());
                }
            }
        }

        Ok(())
    }
}
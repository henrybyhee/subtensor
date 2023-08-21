// The MIT License (MIT)
// Copyright © 2023 Yuma Rao

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
// documentation files (the “Software”), to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or substantial portions of
// the Software.

// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use super::*;
use crate::math::*;
use frame_support::inherent::Vec;
use frame_support::sp_std::vec;
use frame_support::storage::IterableStorageDoubleMap;
use substrate_fixed::types::{I32F32, I64F64};

impl<T: Config> Pallet<T> {
    /// Retrieves the unique identifier (UID) for the root network.
    ///
    /// The root network is a special case and has a fixed UID of 0.
    ///
    /// # Returns:
    /// * `u16`: The UID for the root network.
    ///
    pub fn get_root_netuid() -> u16 {
        0
    }

    /// Retrieves the emission setting tempo for the root network.
    ///
    /// The tempo determines how many blocks progress before subnet emissions are recalculated.
    ///
    /// # Returns:
    /// * `u16`: The tempo for the root network.
    ///
    pub fn get_root_tempo() -> u16 {
        100
    }

    /// Fetches the total count of subnets.
    ///
    /// This function retrieves the total number of subnets present on the chain.
    ///
    /// # Returns:
    /// * `u16`: The total number of subnets.
    ///
    pub fn get_num_subnets() -> u16 {
        TotalNetworks::<T>::get()
    }

    /// Gets the maximum permissible number of subnets.
    ///
    /// This function retrieves the hard cap on the number of subnets that can exist.
    ///
    /// # Returns:
    /// * `u16`: The maximum number of allowed subnets.
    ///
    pub fn get_max_allowed_subnets() -> u16 {
        SubnetLimit::<T>::get()
    }

    /// Checks for any UIDs in the given list that are either equal to the root netuid or exceed the total number of subnets.
    ///
    /// It's important to check for invalid UIDs to ensure data integrity and avoid referencing nonexistent subnets.
    ///
    /// # Arguments:
    /// * `uids`: A reference to a vector of UIDs to check.
    ///
    /// # Returns:
    /// * `bool`: `true` if any of the UIDs are invalid, `false` otherwise.
    ///
    pub fn contains_invalid_root_uids(uids: &Vec<u16>) -> bool {
        let total_subnets: u16 = Self::get_num_subnets();
        for uid in uids {
            // Check if the UID exceeds the total number of subnets or matches the root netuid.
            if *uid > total_subnets || *uid == Self::get_root_netuid() {
                return true;
            }
        }
        false
    }

    /// Retrieves weight matrix associated with the root network.
    ///  Weights represent the preferences for each subnetwork.
    ///
    /// # Returns:
    /// A 2D vector (`Vec<Vec<I32F32>>`) where each entry [i][j] represents the weight of subnetwork
    /// `j` with according to the preferences of key. `j` within the root network.
    ///
    pub fn get_root_weights() -> Vec<Vec<I32F32>> {
        // --- 0. Get the size of the root network (the number of registered keys.)
        let n: usize = Self::get_subnetwork_n(Self::get_root_netuid()) as usize;

        // --- 1. Get the total number of subnets.
        let k: usize = Self::get_num_subnets() as usize;

        // --- 2. Initialize a 2D vector with zeros to store the weights. The dimensions are determined
        // by `n` (number of registered key under  root) and `k` (total number of subnets).
        let mut weights: Vec<Vec<I32F32>> = vec![vec![I32F32::from_num(0.0); k]; n];

        // --- 3. Iterate over stored weights and fill the matrix.
        for (uid_i, weights_i) in
            <Weights<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)>>>::iter_prefix(
                Self::get_root_netuid(),
            )
        {
            // --- 4. Iterate over each weight entry in `weights_i` to update the corresponding value in the
            // initialized `weights` 2D vector. Here, `uid_j` represents a subnet, and `weight_ij` is the
            // weight of `uid_i` with respect to `uid_j`.
            for (uid_j, weight_ij) in weights_i.iter() {
                weights[uid_i as usize][*uid_j as usize] = I32F32::from_num(*weight_ij);
            }
        }

        // --- 5. Return the filled weights matrix.
        weights
    }

    /// Computes and sets emission values for the root network which determine the emission for all subnets.
    ///
    /// This function is responsible for calculating emission based on network weights, stake values,
    /// and registered hotkeys.
    ///
    pub fn root_epoch(block_number: u64) {
        // --- -1. Check if we should update the emission values based on blocks since emission was last set.
        if Self::blocks_until_next_epoch(
            Self::get_root_netuid(),
            Self::get_root_tempo(),
            block_number,
        ) > 0
        {
            return;
        }

        // --- 0. The unique ID associated with the root network.
        let root_netuid: u16 = Self::get_root_netuid();

        // --- 1. Retrieves the number of registered peers on the the root network.
        let n: u16 = Self::get_subnetwork_n(root_netuid);
        log::trace!("n:\n{:?}\n", n);

        // --- 2. Obtains the maximum number of registered subnetworks. This function
        // will return a vector of size k.
        let k: u16 = Self::get_num_subnets();
        log::trace!("k:\n{:?}\n", k);

        // --- 3. Determines the total block emission across all the subnetworks. This is the
        // value which will be distributed based on the computation below.
        let emission: u64 = Self::get_block_emission();
        log::trace!("emission:\n{:?}\n", emission);

        // --- 4. A collection of all registered hotkeys on the root network. Hotkeys
        // pairs with network UIDs and stake values.
        let mut hotkeys: Vec<(u16, T::AccountId)> = vec![];
        for (uid_i, hotkey) in
            <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix(root_netuid)
        {
            hotkeys.push((uid_i, hotkey));
        }
        log::trace!("hotkeys:\n{:?}\n", hotkeys);

        // --- 5. Retrieves and stores the stake value associated with each hotkey on the root network.
        // Stakes are stored in a 64-bit fixed point representation for precise calculations.
        let mut stake_i64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
        for (uid_i, hotkey) in hotkeys.iter() {
            stake_i64[*uid_i as usize] = I64F64::from_num(Self::get_total_stake_for_hotkey(hotkey));
        }
        inplace_normalize_64(&mut stake_i64);

        // --- 6. Converts the 64-bit fixed point stake values to float 32-bit for ease of further calculations.
        let stake_i32: Vec<I32F32> = vec_fixed64_to_fixed32(stake_i64);
        log::trace!("S:\n{:?}\n", &stake_i32);

        // --- 7. Retrieves the network weights in a 2D Vector format. Weights have shape
        // n x k where is n is the number of registered peers and k is the number of subnets.
        let weights_i32: Vec<Vec<I32F32>> = Self::get_weights(root_netuid);
        log::trace!("W:\n{:?}\n", &weights_i32);

        // --- 8. Calculates the rank of networks. Rank is a product of weights and stakes.
        // Ranks will have shape k, a score for each subnet.
        let ranks_i32: Vec<I32F32> = matmul(&weights_i32, &stake_i32);
        log::trace!("R:\n{:?}\n", &ranks_i32);

        // --- 9. Converts the rank values to 64-bit fixed point representation for normalization.
        let mut emission_i62: Vec<I64F64> = vec_fixed32_to_fixed64(ranks_i32);
        inplace_normalize_64(&mut emission_i62);
        log::trace!("Ei64:\n{:?}\n", &emission_i62);

        // --- 10. Converts the normalized 64-bit fixed point rank values to u64 for the final emission calculation.
        let emission_u64: Vec<u64> = vec_fixed64_to_u64(emission_i62);
        log::trace!("Eu64:\n{:?}\n", &emission_u64);

        // --- 11. Set the emission values for each subnet directly.
        for (netuid_i, emission_i) in emission_u64.iter().enumerate() {
            Self::set_emission_for_network(netuid_i as u16, *emission_i);
        }
    }

    // ---- The implementation for the extrinsic set_root_weights.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling hotkey.
    //
    // 	* 'uids' ( Vec<u16> ):
    // 		- The uids of the weights to be set on the chain.
    //
    // 	* 'values' ( Vec<u16> ):
    // 		- The values of the weights to set on the chain.
    //
    // # Event:
    // 	* WeightsSet;
    // 		- On successfully setting the weights on chain.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'SettingWeightsTooFast':
    // 		- Attempting to set weights faster than the weights_set_rate_limit.
    //
    // 	* 'WeightVecNotEqualSize':
    // 		- Attempting to set weights with uids not of same length.
    //
    // 	* 'DuplicateUids':
    // 		- Attempting to set weights with duplicate uids.
    //
    // 	* 'InvalidUid':
    // 		- Attempting to set weights with invalid uids.
    //
    pub fn set_root_weights(
        origin: T::RuntimeOrigin,
        uids: Vec<u16>,
        values: Vec<u16>,
    ) -> dispatch::DispatchResult {
        let root_netuid: u16 = Self::get_root_netuid();

        // --- 1. Check the caller's signature. This is the hotkey of a registered account.
        let hotkey = ensure_signed(origin)?;
        log::info!(
            "do_set_root_weights( origin:{:?} uids:{:?}, values:{:?})",
            hotkey,
            uids,
            values
        );

        // --- 2. Check that the length of uid list and value list are equal for this network.
        ensure!(
            Self::uids_match_values(&uids, &values),
            Error::<T>::WeightVecNotEqualSize
        );

        // --- 3. Check to see if the number of uids is within the max allowed uids for this network.
        ensure!(
            uids.len() <= TotalNetworks::<T>::get() as usize,
            Error::<T>::TooManyUids
        );

        // --- 4. Ensure the hotkey is registered on the root network.
        ensure!(
            Self::is_hotkey_registered_on_network(root_netuid, &hotkey),
            Error::<T>::NotRegistered
        );

        // --- 5. Get the neuron uid of associated hotkey on network netuid.
        let neuron_uid;
        let net_neuron_uid = Self::get_uid_for_net_and_hotkey(root_netuid, &hotkey);
        ensure!(
            net_neuron_uid.is_ok(),
            net_neuron_uid
                .err()
                .unwrap_or(Error::<T>::NotRegistered.into())
        );
        neuron_uid = net_neuron_uid.unwrap();

        // --- 6. Ensure the uid is not setting weights faster than the weights_set_rate_limit.
        let current_block: u64 = Self::get_current_block_as_u64();
        ensure!(
            Self::check_rate_limit(root_netuid, neuron_uid, current_block),
            Error::<T>::SettingWeightsTooFast
        );

        // --- 7. Ensure the passed uids contain no duplicates.
        ensure!(!Self::has_duplicate_uids(&uids), Error::<T>::DuplicateUids);

        // --- 8. Ensure that the passed uids are valid for the network.
        ensure!(
            !Self::contains_invalid_root_uids(&uids),
            Error::<T>::InvalidUid
        );

        // --- 9. Max-upscale the weights.
        let max_upscaled_weights: Vec<u16> = vec_u16_max_upscale_to_u16(&values);

        // --- 10. Zip weights for sinking to storage map.
        let mut zipped_weights: Vec<(u16, u16)> = vec![];
        for (uid, val) in uids.iter().zip(max_upscaled_weights.iter()) {
            zipped_weights.push((*uid, *val))
        }

        // --- 11. Set weights under netuid, uid double map entry.
        Weights::<T>::insert(root_netuid, neuron_uid, zipped_weights);

        // --- 12. Set the activity for the weights on this network.
        Self::set_last_update_for_uid(root_netuid, neuron_uid, current_block);

        // --- 13. Emit the tracking event.
        log::info!(
            "RootWeightsSet( root_netuid:{:?}, neuron_uid:{:?} )",
            root_netuid,
            neuron_uid
        );
        Self::deposit_event(Event::WeightsSet(root_netuid, neuron_uid));

        // --- 14. Return ok.
        Ok(())
    }

    /// Registers a user's hotkey to the root network.
    ///
    /// This function is responsible for registering the hotkey of a user.
    /// The root key with the least stake if pruned in the event of a filled network.
    ///
    /// # Arguments:
    /// * `origin`: Represents the origin of the call.
    /// * `hotkey`: The hotkey that the user wants to register to the root network.
    ///
    /// # Returns:
    /// * `DispatchResult`: A result type indicating success or failure of the registration.
    ///
    pub fn do_root_register(origin: T::RuntimeOrigin, hotkey: T::AccountId) -> DispatchResult {
        // --- 0. Get the unique identifier (UID) for the root network.
        let root_netuid: u16 = Self::get_root_netuid();
        let current_block_number: u64 = Self::get_current_block_as_u64();
        ensure!(
            Self::if_subnet_exist(root_netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 1. Ensure that the call originates from a signed source and retrieve the caller's account ID (coldkey).
        let coldkey = ensure_signed(origin)?;
        log::info!(
            "do_root_register( coldkey: {:?}, hotkey: {:?} )",
            coldkey,
            hotkey
        );

        // --- 2. Ensure that the number of registrations in this block doesn't exceed the allowed limit.
        ensure!(
            Self::get_registrations_this_block(root_netuid)
                < Self::get_max_registrations_per_block(root_netuid),
            Error::<T>::TooManyRegistrationsThisBlock
        );

        // --- 3. Ensure that the number of registrations in this interval doesn't exceed thrice the target limit.
        ensure!(
            Self::get_registrations_this_interval(root_netuid)
                < Self::get_target_registrations_per_interval(root_netuid) * 3,
            Error::<T>::TooManyRegistrationsThisInterval
        );

        // --- 4. Check if the hotkey is already registered. If so, error out.
        ensure!(
            !Uids::<T>::contains_key(root_netuid, &hotkey),
            Error::<T>::AlreadyRegistered
        );

        // --- 6. Create a network account for the user if it doesn't exist.
        Self::create_account_if_non_existent(&coldkey, &hotkey);

        // --- 7. Fetch the current size of the subnetwork.
        let current_subnetwork_n: u16 = Self::get_subnetwork_n(root_netuid);
        ensure!(
            Self::get_max_allowed_uids(root_netuid) != 0,
            Error::<T>::NetworkDoesNotExist
        );

        // Declare a variable to hold the root UID.
        let subnetwork_uid: u16;

        // --- 8. Check if the root net is below its allowed size.
        if current_subnetwork_n < Self::get_max_allowed_uids(root_netuid) {
            // --- 12.1.1 We can append to the subnetwork as it's not full.
            subnetwork_uid = current_subnetwork_n;

            // --- 12.1.2 Add the new account and make them a member of the Senate.
            Self::append_neuron(root_netuid, &hotkey, current_block_number);
            log::info!("add new neuron account");
        } else {
            // --- 13.1.1 The network is full. Perform replacement.
            // Find the neuron with the lowest stake value to replace.
            let mut lowest_stake: u64 = u64::MAX;
            let mut lowest_uid: u16 = 0;

            // Iterate over all keys in the root network to find the neuron with the lowest stake.
            for (uid_i, hotkey_i) in
                <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix(
                    root_netuid,
                )
            {
                let stake_i: u64 = Self::get_total_stake_for_hotkey(&hotkey_i);
                if stake_i < lowest_stake {
                    lowest_stake = stake_i;
                    lowest_uid = uid_i;
                }
            }
            subnetwork_uid = lowest_uid;

            // --- 13.1.2 The new account has a higher stake than the one being replaced.
            ensure!(
                lowest_stake < Self::get_total_stake_for_hotkey(&hotkey),
                Error::<T>::StakeTooLowForRoot
            );

            // --- 13.1.3 The new account has a higher stake than the one being replaced.
            // Replace the neuron account with new information.
            Self::replace_neuron(root_netuid, lowest_uid, &hotkey, current_block_number);
            log::info!("replace neuron");
        }

        // --- 14. Update the registration counters for both the block and interval.
        RegistrationsThisInterval::<T>::mutate(root_netuid, |val| *val += 1);
        RegistrationsThisBlock::<T>::mutate(root_netuid, |val| *val += 1);

        // --- 15. Log and announce the successful registration.
        log::info!(
            "RootRegistered(netuid:{:?} uid:{:?} hotkey:{:?})",
            root_netuid,
            subnetwork_uid,
            hotkey
        );
        Self::deposit_event(Event::NeuronRegistered(root_netuid, subnetwork_uid, hotkey));

        // --- 16. Finish and return success.
        Ok(())
    }

    /// Facilitates user registration of a new subnetwork.
    ///
    /// # Args:
    /// 	* `origin`: (`T::RuntimeOrigin`): The calling origin. Must be signed.
    ///
    /// # Event:
    /// 	* `NetworkAdded`: Emitted when a new network is successfully added.
    ///
    /// # Raises:
    /// 	* `TxRateLimitExceeded`: If the rate limit for network registration is exceeded.
    /// 	* `NotEnoughBalanceToStake`: If there isn't enough balance to stake for network registration.
    /// 	* `BalanceWithdrawalError`: If an error occurs during balance withdrawal for network registration.
    ///
    pub fn user_add_network(origin: T::RuntimeOrigin) -> dispatch::DispatchResult {
        // --- 0. Ensure the caller is a signed user.
        let coldkey = ensure_signed(origin)?;

        // --- 1. Rate limit for network registrations.
        let current_block = Self::get_current_block_as_u64();
        let last_burn_block = Self::get_network_last_burn_block();
        ensure!(
            current_block - last_burn_block >= 1, // Replace 1 with a configurable time limit if desired.
            Error::<T>::TxRateLimitExceeded
        );

        // --- 2. Calculate and lock the required tokens.
        let lock_amount: u64 = Self::get_network_burn_cost();
        let lock_as_balance = Self::u64_to_balance(lock_amount);
        ensure!(
            lock_as_balance.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );
        ensure!(
            Self::can_remove_balance_from_coldkey_account(&coldkey, lock_as_balance.unwrap()),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 3. Fetch current and maximum subnets.
        let current_num_subnets: u16 = Self::get_num_subnets();
        let max_allowed_subnets: u16 = Self::get_max_allowed_subnets();

        // --- 4. Determine the netuid to register.
        let netuid_to_register: u16 = {
            if current_num_subnets < max_allowed_subnets {
                let mut next_available_netuid = 0;
                loop {
                    next_available_netuid += 1;
                    if !Self::if_subnet_exist(next_available_netuid) {
                        break next_available_netuid;
                    }
                }
            } else {
                let netuid_to_prune = Self::get_subnet_to_prune();
                Self::remove_network(netuid_to_prune);
                netuid_to_prune
            }
        };

        // --- 5. Perform the lock operation.
        ensure!(
            Self::remove_balance_from_coldkey_account(&coldkey, lock_as_balance.unwrap()) == true,
            Error::<T>::BalanceWithdrawalError
        );
        Self::set_subnet_locked_balance(netuid_to_register, lock_amount);

        // --- 6. Set initial and custom parameters for the network.
        Self::init_new_network_with_params(netuid_to_register);

        // --- 7. Set netuid storage.
        NetworkLastRegistered::<T>::set(current_block);
        NetworkRegisteredAt::<T>::insert(netuid_to_register, current_block);
        SubnetOwner::<T>::insert(netuid_to_register, coldkey);
        Self::set_network_last_burn(lock_amount);

        // --- 8. Emit the NetworkAdded event.
        log::info!(
            "NetworkAdded( netuid:{:?}, modality:{:?} )",
            netuid_to_register,
            0
        );
        Self::deposit_event(Event::NetworkAdded(netuid_to_register, 0));

        // --- 9. Return success.
        Ok(())
    }

    /// Sets initial and custom parameters for a new network.
    fn init_new_network_with_params(netuid: u16) {
        Self::init_new_network(netuid, 100, 0);
        Self::set_network_registration_allowed(netuid, true);
        Self::set_immunity_period(netuid, 1000);
        Self::set_max_allowed_uids(netuid, 256);
        Self::set_max_allowed_validators(netuid, 128);
        Self::set_min_allowed_weights(netuid, 64);
        Self::set_max_weight_limit(netuid, 511);
        Self::set_adjustment_interval(netuid, 500);
        Self::set_target_registrations_per_interval(netuid, 1);
        Self::set_adjustment_alpha(netuid, 58000);
        Self::set_immunity_period(netuid, 5000);
        Self::set_min_burn(netuid, 100_000_000);
    }
}

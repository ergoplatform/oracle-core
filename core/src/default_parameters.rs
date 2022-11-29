//! Default parameter values for all Oracle-pool contracts. Tracks values described in EIP-0023.

use std::convert::TryInto;

use ergo_lib::{
    ergo_chain_types::blake2b256_hash, ergotree_ir::chain::ergo_box::box_value::BoxValue,
};

use crate::contracts::{
    ballot::BallotContractParameters,
    oracle::OracleContractParameters,
    pool::PoolContractParameters,
    refresh::{RefreshContractParameters, RefreshContractParametersInputs},
    update::UpdateContractParameters,
};

impl Default for BallotContractParameters {
    fn default() -> Self {
        // compiled via
        // https://scastie.scala-lang.org/P977Sr4qTKylV427dIP75Q
        let ergo_tree_bytes = base16::decode("10070580dac409040204020400040204000e206251655468576d5a7134743777217a25432a462d4a404e635266556a586e3272d803d601b2a5e4e3000400d602c672010407d603e4c6a70407ea02d1ededede6720293c27201c2a793db63087201db6308a792c172017300eb02cd7203d1ededededed91b1a4730191b1db6308b2a47302007303938cb2db6308b2a473040073050001730693e47202720392c17201c1a7efe6c672010561").unwrap();
        let min_storage_rent_index = 0;
        let min_storage_rent: BoxValue = 10000000u64.try_into().unwrap();
        let update_nft_index = 6;
        BallotContractParameters::checked_load(
            ergo_tree_bytes,
            min_storage_rent,
            min_storage_rent_index,
            update_nft_index,
        )
        .unwrap()
    }
}

impl Default for OracleContractParameters {
    fn default() -> Self {
        // compiled via
        // https://scastie.scala-lang.org/Ub0eB9H7TOuPgq6sAf4cMQ
        let ergo_tree_bytes = base16::decode("100a040004000580dac409040004000e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570402040204020402d804d601b2a5e4e3000400d602db63087201d603db6308a7d604e4c6a70407ea02d1ededed93b27202730000b2720373010093c27201c2a7e6c67201040792c172017302eb02cd7204d1ededededed938cb2db6308b2a4730300730400017305938cb27202730600018cb2720373070001918cb27202730800028cb272037309000293e4c672010407720492c17201c1a7efe6c672010561").unwrap();
        let pool_nft_index = 5;
        let min_storage_rent_index = 2;
        let min_storage_rent = 10000000u64.try_into().unwrap();
        OracleContractParameters::checked_load(
            ergo_tree_bytes,
            pool_nft_index,
            min_storage_rent_index,
            min_storage_rent,
        )
        .unwrap()
    }
}

impl Default for PoolContractParameters {
    fn default() -> Self {
        // compiled via
        // https://scastie.scala-lang.org/D7lDlGpjRNK5XL9eXKWMKQ
        let ergo_tree_bytes = base16::decode("1004040204000e20546a576e5a7234753778214125442a472d4b614e645267556b587032733576380e206251655468576d5a7134743777217a25432a462d4a404e635266556a586e3272d801d6018cb2db6308b2a473000073010001d1ec93720173029372017303").unwrap();

        let refresh_nft_index = 2;
        let update_nft_index = 3;
        PoolContractParameters::checked_load(ergo_tree_bytes, refresh_nft_index, update_nft_index)
            .unwrap()
    }
}

impl Default for RefreshContractParameters {
    fn default() -> Self {
        // compiled via
        // https://scastie.scala-lang.org/Uxx4eebYQFqg7KZ0F29TTg
        let ergo_tree_bytes = base16::decode("1016043c040004000e202a472d4a614e645267556b58703273357638792f423f4528482b4d625065536801000502010105000400040004020402040204080400040a05c8010e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570400040404020408d80ed60199a37300d602b2a4730100d603b5a4d901036395e6c672030605eded928cc77203017201938cb2db6308720373020001730393e4c672030504e4c6720205047304d604b17203d605b0720386027305860273067307d901053c413d0563d803d607e4c68c7205020605d6088c720501d6098c720802860272078602ed8c720901908c72080172079a8c7209027207d6068c720502d6078c720501d608db63087202d609b27208730800d60ab2a5730900d60bdb6308720ad60cb2720b730a00d60db27208730b00d60eb2a5730c00ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02cde4c6b27203e4e30004000407d18f8cc77202017201d1927204730dd18c720601d190997207e4c6b27203730e0006059d9c72077e730f057310d1938c7209017311d193b2720b7312007209d1938c720c018c720d01d1928c720c02998c720d027e9c7204731305d193b1720bb17208d193e4c6720a04059d8c7206027e720405d193e4c6720a05049ae4c6720205047314d193c2720ac27202d192c1720ac17202d1928cc7720a0199a37315d193db6308720edb6308a7d193c2720ec2a7d192c1720ec1a7").unwrap();
        RefreshContractParameters::checked_load(RefreshContractParametersInputs {
            ergo_tree_bytes,
            pool_nft_index: 17,
            oracle_token_id_index: 3,
            min_data_points_index: 13,
            min_data_points: 4,
            buffer_length_index: 21,
            buffer_length: 4,
            max_deviation_percent_index: 15,
            max_deviation_percent: 5,
            epoch_length_index: 0,
            epoch_length: 30,
        })
        .unwrap()
    }
}

impl Default for UpdateContractParameters {
    fn default() -> Self {
        // compiled via
        // https://scastie.scala-lang.org/BxJFCRDcSCeiwf9ZgKdymQ
        let ergo_tree_bytes = base16::decode("100e040004000400040204020e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570400040004000e203f4428472d4b6150645367566b5970337336763979244226452948404d625165010005000400040cd806d601b2a4730000d602b2db63087201730100d603b2a5730200d604db63087203d605b2a5730300d606b27204730400d1ededed938c7202017305edededed937202b2720473060093c17201c1720393c672010405c67203040593c672010504c672030504efe6c672030661edededed93db63087205db6308a793c27205c2a792c17205c1a7918cc77205018cc7a701efe6c67205046192b0b5a4d9010763d801d609db630872079591b172097307edededed938cb2720973080001730993e4c6720705048cc7a70193e4c67207060ecbc2720393e4c67207070e8c72060193e4c6720708058c720602730a730bd9010741639a8c7207018cb2db63088c720702730c00027e730d05").unwrap();
        let pool_nft_index = 5;
        let ballot_token_index = 9;
        let min_votes_index = 13;
        let min_votes = 6;
        UpdateContractParameters::checked_load(
            ergo_tree_bytes,
            pool_nft_index,
            ballot_token_index,
            min_votes_index,
            min_votes,
        )
        .unwrap()
    }
}

pub fn print_contract_hashes() {
    let encoded_hash = |bytes| base64::encode(blake2b256_hash(bytes));

    println!("BASE 64 ENCODING OF BLAKE2B HASH OF CONTRACT ERGO-TREE BYTES");
    println!("------------------------------------------------------------\n");

    let pool_ergo_tree_bytes = &PoolContractParameters::default().ergo_tree_bytes();

    println!(
        "Pool contract encoded hash: {}",
        encoded_hash(pool_ergo_tree_bytes)
    );

    let refresh_ergo_tree_bytes = &RefreshContractParameters::default().ergo_tree_bytes();

    println!(
        "Refresh contract encoded hash: {}",
        encoded_hash(refresh_ergo_tree_bytes)
    );

    let oracle_ergo_tree_bytes = &OracleContractParameters::default().ergo_tree_bytes();
    println!(
        "Oracle contract encoded hash: {}",
        encoded_hash(oracle_ergo_tree_bytes)
    );

    let ballot_ergo_tree_bytes = &BallotContractParameters::default().ergo_tree_bytes();

    println!(
        "Ballot contract encoded hash: {}",
        encoded_hash(ballot_ergo_tree_bytes)
    );

    let update_ergo_tree_bytes = &UpdateContractParameters::default().ergo_tree_bytes();

    println!(
        "Update contract encoded hash: {}\n",
        encoded_hash(update_ergo_tree_bytes)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_contract_hashes() {
        let encoded_hash = |bytes| base64::encode(blake2b256_hash(bytes));

        let expected_pool_encoding = "8cJi+FGGU32jXyO8M2LeyWSWlerdcb1zxBWeZtyy7Y8=";
        let expected_refresh_encoding = "cs5c5QEirstI4ZlTyrbTjlPwWYHRW+QsedtpyOSBnH4=";
        let expected_oracle_encoding = "fhOYLO3s+NJCqTQDWUz0E+ffy2T1VG7ZnhSFs0RP948=";
        let expected_ballot_encoding = "2DnK+72bh+TxviNk8XfuYzLKtuF5jnqUJOzimt30NvI=";
        let expected_update_encoding = "3aIVTP5tRgCZHxaT3ZFw3XubRV5DJi0rKeo9bKVHlVw=";

        println!("BASE 64 ENCODING OF BLAKE2B HASH OF CONTRACT ERGO-TREE BYTES");
        println!("------------------------------------------------------------\n");

        let pool_ergo_tree_bytes = &PoolContractParameters::default().ergo_tree_bytes();

        let encoded = encoded_hash(pool_ergo_tree_bytes);
        println!("Pool contract encoded hash: {}", encoded,);

        assert_eq!(
            encoded, expected_pool_encoding,
            "Differing pool contract hash, expected {}, got {}",
            encoded, expected_pool_encoding
        );

        let refresh_ergo_tree_bytes = &RefreshContractParameters::default().ergo_tree_bytes();

        let encoded = encoded_hash(refresh_ergo_tree_bytes);
        println!("Refresh contract encoded hash: {}", encoded,);
        assert_eq!(
            encoded, expected_refresh_encoding,
            "Differing refresh contract hash, expected {}, got {}",
            encoded, expected_refresh_encoding
        );

        let oracle_ergo_tree_bytes = &OracleContractParameters::default().ergo_tree_bytes();

        let encoded = encoded_hash(oracle_ergo_tree_bytes);
        println!("Oracle contract encoded hash: {}", encoded);
        assert_eq!(
            encoded, expected_oracle_encoding,
            "Differing oracle contract hash, expected {}, got {}",
            encoded, expected_oracle_encoding,
        );

        let ballot_ergo_tree_bytes = &BallotContractParameters::default().ergo_tree_bytes();

        let encoded = encoded_hash(ballot_ergo_tree_bytes);
        println!("Ballot contract encoded hash: {}", encoded);
        assert_eq!(
            encoded, expected_ballot_encoding,
            "Differing ballot contract hash, expected {}, got {}",
            encoded, expected_ballot_encoding,
        );

        let update_ergo_tree_bytes = &UpdateContractParameters::default().ergo_tree_bytes();

        let encoded = encoded_hash(update_ergo_tree_bytes);
        println!("Update contract encoded hash: {}\n", encoded);
        assert_eq!(
            encoded, expected_update_encoding,
            "Differing update contract hash, expected {}, got {}",
            encoded, expected_update_encoding,
        );
    }
}

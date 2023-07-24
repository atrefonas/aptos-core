// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::governance::{utils::*, *};
use clap::Subcommand;
use reqwest::Url;

/// Tool for on-chain governance of delegation pools
#[derive(Subcommand)]
pub enum DelegationPoolTool {
    Propose(SubmitProposal),
    Vote(SubmitVote),
}

impl DelegationPoolTool {
    pub async fn execute(self) -> CliResult {
        use DelegationPoolTool::*;
        match self {
            Propose(tool) => tool.execute_serialized().await,
            Vote(tool) => tool.execute_serialized().await,
        }
    }
}

/// Submit a governance proposal
#[derive(Parser)]
pub struct SubmitProposal {
    /// The address of the delegation pool to propose.
    #[clap(long)]
    delegation_pool_address: AccountAddress,

    /// Location of the JSON metadata of the proposal
    ///
    /// If this location does not keep the metadata in the exact format, it will be less likely
    /// that voters will approve this proposal, as they won't be able to verify it.
    #[clap(long)]
    pub(crate) metadata_url: Url,

    #[cfg(feature = "no-upload-proposal")]
    /// A JSON file to be uploaded later at the metadata URL
    ///
    /// If this does not match properly, voters may choose to vote no.  For real proposals,
    /// it is better to already have it uploaded at the URL.
    #[clap(long)]
    pub(crate) metadata_path: Option<PathBuf>,

    #[clap(long)]
    pub(crate) is_multi_step: bool,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProposalSubmissionSummary {
    proposal_id: Option<u64>,
    txn_summaries: Vec<TransactionSummary>,
}

impl SubmitProposalUtils for SubmitProposal {
    fn get_metadata_url(&self) -> Url {
        self.metadata_url.clone()
    }

    #[cfg(feature = "no-upload-proposal")]
    fn get_metadata_path(&self) -> Option<PathBuf> {
        self.metadata_path.clone()
    }

    fn get_compile_proposal_args(&self) -> &CompileScriptFunction {
        &self.compile_proposal_args
    }

    fn get_txn_options(&self) -> &TransactionOptions {
        &self.txn_options
    }
}

#[async_trait]
impl CliCommand<ProposalSubmissionSummary> for SubmitProposal {
    fn command_name(&self) -> &'static str {
        "SubmitProposal"
    }

    async fn execute(mut self) -> CliTypedResult<ProposalSubmissionSummary> {
        let mut summaries = vec![];
        if let Some(txn_summary) =
            delegation_pool_governance_precheck(&self.txn_options, self.delegation_pool_address)
                .await?
        {
            summaries.push(txn_summary);
        };
        // Validate the proposal metadata
        let (script_hash, metadata_hash) = self.compile_proposals().await?;
        prompt_yes_with_override(
            "Do you want to submit this proposal?",
            self.txn_options.prompt_options,
        )?;

        let txn: Transaction = if self.is_multi_step {
            self.txn_options
                .submit_transaction(aptos_stdlib::delegation_pool_create_proposal(
                    self.delegation_pool_address,
                    script_hash.to_vec(),
                    self.metadata_url.to_string().as_bytes().to_vec(),
                    metadata_hash.to_hex().as_bytes().to_vec(),
                    true,
                ))
                .await?
        } else {
            self.txn_options
                .submit_transaction(aptos_stdlib::delegation_pool_create_proposal(
                    self.delegation_pool_address,
                    script_hash.to_vec(),
                    self.metadata_url.to_string().as_bytes().to_vec(),
                    metadata_hash.to_hex().as_bytes().to_vec(),
                    false,
                ))
                .await?
        };
        let proposal_id = extract_proposal_id(&txn)?;
        summaries.push(TransactionSummary::from(&txn));
        Ok(ProposalSubmissionSummary {
            proposal_id,
            txn_summaries: summaries,
        })
    }
}

/// Submit a vote on a proposal
#[derive(Parser)]
pub struct SubmitVote {
    /// The address of the delegation pool to vote.
    #[clap(long)]
    delegation_pool_address: AccountAddress,

    /// Id of the proposal to vote on
    #[clap(long)]
    pub(crate) proposal_id: u64,

    /// Vote to accept the proposal
    #[clap(long, group = "vote")]
    pub(crate) yes: bool,

    /// Vote to reject the proposal
    #[clap(long, group = "vote")]
    pub(crate) no: bool,

    /// Voting power to use for the vote. If not specified, all the voting power will be used.
    #[clap(long)]
    pub(crate) voting_power: Option<u64>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for SubmitVote {
    fn command_name(&self) -> &'static str {
        "SubmitVote"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let vote = parse_vote_option(self.yes, self.no)?;
        let mut summaries: Vec<TransactionSummary> = vec![];
        if let Some(txn_summary) =
            delegation_pool_governance_precheck(&self.txn_options, self.delegation_pool_address)
                .await?
        {
            summaries.push(txn_summary);
        };

        let client = &self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let voter_address = self.txn_options.profile_options.account_address()?;
        let remaining_voting_power = get_remaining_voting_power(
            client,
            self.delegation_pool_address,
            voter_address,
            self.proposal_id,
        )
        .await?;
        if remaining_voting_power == 0 {
            return Err(CliError::CommandArgumentError(
                "Voter has no voting power left on this proposal".to_string(),
            ));
        };
        let voting_power = check_remaining_voting_power(remaining_voting_power, self.voting_power);
        prompt_yes_with_override(
            &format!(
                "Vote {} with voting power = {} from stake pool {} on proposal {}?",
                vote_to_string(vote),
                voting_power,
                self.delegation_pool_address,
                self.proposal_id,
            ),
            self.txn_options.prompt_options,
        )?;
        summaries.push(
            self.txn_options
                .submit_transaction(aptos_stdlib::delegation_pool_vote(
                    self.delegation_pool_address,
                    self.proposal_id,
                    voting_power,
                    vote,
                ))
                .await
                .map(TransactionSummary::from)?,
        );

        Ok(summaries)
    }
}

/// Precheck before any delegation pool governance operations. Check if feature flags are enabled.
/// Also check if partial governance voting is enabled for delegation pool. If not, send a
/// transaction to enable it.
async fn delegation_pool_governance_precheck(
    txn_options: &TransactionOptions,
    pool_address: AccountAddress,
) -> CliTypedResult<Option<TransactionSummary>> {
    let client = &txn_options
        .rest_options
        .client(&txn_options.profile_options)?;
    if !is_partial_governance_voting_enabled(client).await? {
        return Err(CliError::CommandArgumentError(
            "Partial governance voting feature flag is not enabled".to_string(),
        ));
    };
    if !is_delegation_pool_partial_governance_voting_enabled(client).await? {
        return Err(CliError::CommandArgumentError(
            "Delegation pool partial governance voting feature flag is not enabled".to_string(),
        ));
    };
    if is_partial_governance_voting_enabled_for_delegation_pool(client, pool_address).await? {
        Ok(None)
    } else {
        println!("Partial governance voting for delegation pool {} hasn't been enabled yet. Enabling it now...",
                 pool_address);
        let txn_summary = txn_options
            .submit_transaction(
                aptos_stdlib::delegation_pool_enable_partial_governance_voting(pool_address),
            )
            .await
            .map(TransactionSummary::from)?;
        Ok(Some(txn_summary))
    }
}

async fn is_partial_governance_voting_enabled_for_delegation_pool(
    client: &Client,
    pool_address: AccountAddress,
) -> CliTypedResult<bool> {
    let response = client
        .view(
            &ViewRequest {
                function: "0x1::delegation_pool::partial_governance_voting_enabled"
                    .parse()
                    .unwrap(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::String(pool_address.to_string())],
            },
            None,
        )
        .await?;
    Ok(response.inner()[0].as_bool().unwrap())
}

async fn get_remaining_voting_power(
    client: &Client,
    pool_address: AccountAddress,
    voter_address: AccountAddress,
    proposal_id: u64,
) -> CliTypedResult<u64> {
    let response = client
        .view(
            &ViewRequest {
                function: "0x1::delegation_pool::calculate_and_update_remaining_voting_power"
                    .parse()
                    .unwrap(),
                type_arguments: vec![],
                arguments: vec![
                    serde_json::Value::String(pool_address.to_string()),
                    serde_json::Value::String(voter_address.to_string()),
                    serde_json::Value::String(proposal_id.to_string()),
                ],
            },
            None,
        )
        .await?;
    Ok(response.inner()[0].as_str().unwrap().parse().unwrap())
}

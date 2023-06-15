import { AptosConfig } from "./aptos_config";
import { get, post } from "../client";
import { Block, IndexResponse, MoveValue, ViewRequest } from "../types";
import { Memoize, parseApiError } from "../utils";

export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries the latest ledger information
   * @returns Latest ledger information
   * @example Example of returned data
   * ```
   * {
   *   chain_id: 15,
   *   epoch: 6,
   *   ledgerVersion: "2235883",
   *   ledger_timestamp:"1654580922321826"
   * }
   * ```
   */
  @parseApiError
  async getLedgerInfo(): Promise<IndexResponse> {
    const { data } = await get<{}, IndexResponse>({
      url: this.config.network,
      originMethod: "getLedgerInfo",
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * @returns Current chain id
   */
  @Memoize()
  async getChainId(): Promise<number> {
    const { data } = await get<{}, IndexResponse>({
      url: this.config.network,
      originMethod: "getChainId",
      overrides: { ...this.config.clientConfig },
    });
    return data.chain_id;
  }

  /**
   * Call for a move view function
   *
   * @param payload Transaction payload
   * @param version (optional) Ledger version to lookup block information for
   *
   * @returns MoveValue[]
   */
  @parseApiError
  async view(payload: ViewRequest, ledger_version?: string): Promise<MoveValue[]> {
    const { data } = await post<ViewRequest, MoveValue[]>({
      url: this.config.network,
      body: payload,
      endpoint: "view",
      originMethod: "view",
      params: { ledger_version },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * Get block by height
   *
   * @param blockHeight Block height to lookup.  Starts at 0
   * @param withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  @parseApiError
  async getBlockByHeight(blockHeight: number, withTransactions?: boolean): Promise<Block> {
    const { data } = await get<{}, Block>({
      url: this.config.network,
      endpoint: `blocks/by_height/${blockHeight}`,
      originMethod: "getBlockByHeight",
      params: { with_transactions: withTransactions },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * Get block by block transaction version
   *
   * @param version Ledger version to lookup block information for
   * @param withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  @parseApiError
  async getBlockByVersion(version: number, withTransactions?: boolean): Promise<Block> {
    const { data } = await get<{}, Block>({
      url: this.config.network,
      endpoint: `blocks/by_version/${version}`,
      originMethod: "getBlockByVersion",
      params: { with_transactions: withTransactions },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }
}

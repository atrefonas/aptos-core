import { AptosConfig } from "./aptos_config";
import { get } from "../client";
import {
  AnyNumber,
  HexString,
  MaybeHexString,
  PaginationArgs,
  AccountData,
  MoveModuleBytecode,
  MoveResource,
  MoveStructTag,
  Transaction,
} from "../types";
import { parseApiError, paginateWithCursor } from "../utils";

export class Account {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries an Aptos account by address
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @returns Core account resource, used for identifying account and transaction execution
   * @example An example of the returned account
   * ```
   * {
   *    sequence_number: "1",
   *    authentication_key: "0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69"
   * }
   * ```
   */
  @parseApiError
  async getData(accountAddress: MaybeHexString): Promise<AccountData> {
    const { data } = await get<{}, AccountData>({
      url: this.config.network,
      endpoint: `accounts/${HexString.ensure(accountAddress).hex()}`,
      originMethod: "getData",
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * Queries modules associated with given account
   *
   * Note: In order to get all account modules, this function may call the API
   * multiple times as it paginates.
   *
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
   * @returns Account modules array for a specific ledger version.
   * Module is represented by MoveModule interface. It contains module `bytecode` and `abi`,
   * which is JSON representation of a module
   */
  @parseApiError
  async getModules(
    accountAddress: MaybeHexString,
    query?: { ledgerVersion?: AnyNumber },
  ): Promise<MoveModuleBytecode[]> {
    // Note: This function does not expose a `limit` parameter because it might
    // be ambiguous how this is being used. Is it being passed to getAccountModules
    // to limit the number of items per response, or does it limit the total output
    // of this function? We avoid this confusion by not exposing the parameter at all.
    const out = await paginateWithCursor<{}, MoveModuleBytecode[]>({
      url: this.config.network,
      endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/modules`,
      params: { ledger_version: query?.ledgerVersion, limit: 1000 },
      originMethod: "getModules",
      overrides: { ...this.config.clientConfig },
    });
    return out;
  }

  /**
   * Queries module associated with given account by module name
   *
   * Note: In order to get all account resources, this function may call the API
   * multiple times as it paginates.
   *
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param moduleName The name of the module
   * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
   * @returns Specified module.
   * Module is represented by MoveModule interface. It contains module `bytecode` and `abi`,
   * which JSON representation of a module
   */
  @parseApiError
  async getModule(
    accountAddress: MaybeHexString,
    moduleName: string,
    query?: { ledgerVersion?: AnyNumber },
  ): Promise<MoveModuleBytecode> {
    const { data } = await get<{}, MoveModuleBytecode>({
      url: this.config.network,
      endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/module/${moduleName}`,
      originMethod: "getModule",
      params: { ledger_version: query?.ledgerVersion },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * Queries transactions sent by given account
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query Optional pagination object
   * @param query.start The sequence number of the start transaction of the page. Default is 0.
   * @param query.limit The max number of transactions should be returned for the page. Default is 25.
   * @returns An array of on-chain transactions, sent by account
   */
  @parseApiError
  async getTransactions(accountAddress: MaybeHexString, query?: PaginationArgs): Promise<Transaction[]> {
    const { data } = await get<{}, Transaction[]>({
      url: this.config.network,
      endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/transactions`,
      originMethod: "getTransactions",
      params: { start: query?.start, limit: query?.limit },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }

  /**
   * Queries all resources associated with given account
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
   * @returns Account resources for a specific ledger version
   */
  @parseApiError
  async getResources(accountAddress: MaybeHexString, query?: { ledgerVersion?: AnyNumber }): Promise<MoveResource[]> {
    const out = await paginateWithCursor<{}, MoveResource[]>({
      url: this.config.network,
      endpoint: `accounts/${accountAddress}/resources`,
      params: { ledger_version: query?.ledgerVersion, limit: 9999 },
      originMethod: "getResources",
      overrides: { ...this.config.clientConfig },
    });
    return out;
  }

  /**
   * Queries resource associated with given account by resource type
   * @param accountAddress Hex-encoded 32 byte Aptos account address
   * @param resourceType String representation of an on-chain Move struct type
   * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
   * @returns Account resource of specified type and ledger version
   * @example An example of an account resource
   * ```
   * {
   *    type: "0x1::aptos_coin::AptosCoin",
   *    data: { value: 6 }
   * }
   * ```
   */
  @parseApiError
  async getResource(
    accountAddress: MaybeHexString,
    resourceType: MoveStructTag,
    query?: { ledgerVersion?: AnyNumber },
  ): Promise<MoveResource> {
    const { data } = await get<{}, MoveResource>({
      url: this.config.network,
      endpoint: `accounts/${HexString.ensure(accountAddress).hex()}/resource/${resourceType}`,
      originMethod: "getResource",
      params: { ledger_version: query?.ledgerVersion },
      overrides: { ...this.config.clientConfig },
    });
    return data;
  }
}

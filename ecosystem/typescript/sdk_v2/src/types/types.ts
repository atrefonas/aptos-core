// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { bytesToHex, hexToBytes } from "@noble/hashes/utils";
import * as Gen from "./generated";

/**
 * A util class for working with hex strings.
 * Hex strings are strings that are prefixed with `0x`
 */
export class HexString {
  /// We want to make sure this hexString has the `0x` hex prefix
  private readonly hexString: string;

  /**
   * Creates new hex string from Buffer
   * @param buffer A buffer to convert
   * @returns New HexString
   */
  static fromBuffer(buffer: Uint8Array): HexString {
    return HexString.fromUint8Array(buffer);
  }

  /**
   * Creates new hex string from Uint8Array
   * @param arr Uint8Array to convert
   * @returns New HexString
   */
  static fromUint8Array(arr: Uint8Array): HexString {
    return new HexString(bytesToHex(arr));
  }

  /**
   * Ensures `hexString` is instance of `HexString` class
   * @param hexString String to check
   * @returns New HexString if `hexString` is regular string or `hexString` if it is HexString instance
   * @example
   * ```
   *  const regularString = "string";
   *  const hexString = new HexString("string"); // "0xstring"
   *  HexString.ensure(regularString); // "0xstring"
   *  HexString.ensure(hexString); // "0xstring"
   * ```
   */
  static ensure(hexString: MaybeHexString): HexString {
    if (typeof hexString === "string") {
      return new HexString(hexString);
    }
    return hexString;
  }

  /**
   * Creates new HexString instance from regular string. If specified string already starts with "0x" prefix,
   * it will not add another one
   * @param hexString String to convert
   * @example
   * ```
   *  const string = "string";
   *  new HexString(string); // "0xstring"
   * ```
   */
  constructor(hexString: string) {
    if (hexString.startsWith("0x")) {
      this.hexString = hexString;
    } else {
      this.hexString = `0x${hexString}`;
    }
  }

  /**
   * Getter for inner hexString
   * @returns Inner hex string
   */
  hex(): string {
    return this.hexString;
  }

  /**
   * Getter for inner hexString without prefix
   * @returns Inner hex string without prefix
   * @example
   * ```
   *  const hexString = new HexString("string"); // "0xstring"
   *  hexString.noPrefix(); // "string"
   * ```
   */
  noPrefix(): string {
    return this.hexString.slice(2);
  }

  /**
   * Overrides default `toString` method
   * @returns Inner hex string
   */
  toString(): string {
    return this.hex();
  }

  /**
   * Trimmes extra zeroes in the begining of a string
   * @returns Inner hexString without leading zeroes
   * @example
   * ```
   *  new HexString("0x000000string").toShortString(); // result = "0xstring"
   * ```
   */
  toShortString(): string {
    const trimmed = this.hexString.replace(/^0x0*/, "");
    return `0x${trimmed}`;
  }

  /**
   * Converts hex string to a Uint8Array
   * @returns Uint8Array from inner hexString without prefix
   */
  toUint8Array(): Uint8Array {
    return Uint8Array.from(hexToBytes(this.noPrefix()));
  }
}

/**
 * This error is used by `waitForTransactionWithResult` if `checkSuccess` is true.
 * See that function for more information.
 */
export class FailedTransactionError extends Error {
  public readonly transaction: Gen.Transaction;

  constructor(message: string, transaction: Gen.Transaction) {
    super(message);
    this.transaction = transaction;
  }
}

/**
 * This error is used by `waitForTransactionWithResult` when waiting for a
 * transaction times out.
 */
export class WaitForTransactionError extends Error {
  public readonly lastSubmittedTransaction: Gen.Transaction | undefined;

  constructor(message: string, lastSubmittedTransaction: Gen.Transaction | undefined) {
    super(message);
    this.lastSubmittedTransaction = lastSubmittedTransaction;
  }
}

export interface PaginationArgs {
  start?: AnyNumber;
  limit?: number;
}
export type MaybeHexString = HexString | string;

export interface OptionalTransactionArgs {
  maxGasAmount?: Uint64;
  gasUnitPrice?: Uint64;
  expireTimestamp?: Uint64;
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

export type Seq<T> = T[];

export type Uint8 = number;
export type Uint16 = number;
export type Uint32 = number;
export type Uint64 = bigint;
export type Uint128 = bigint;
export type Uint256 = bigint;
export type AnyNumber = bigint | number;
export type Bytes = Uint8Array;

/**
 * A configuration object we can pass with the request to the server.
 *
 * @param TOKEN - an auth token to send with the request
 * @param HEADERS - extra headers we want to send with the request
 * @param WITH_CREDENTIALS - whether to carry cookies. By default, it is set to true and cookies will be sent
 */
export type ClientConfig = {
  TOKEN?: string;
  HEADERS?: Record<string, string | number | boolean>;
  WITH_CREDENTIALS?: boolean;
};

/**
 * The API request type
 *
 * @param url - the url to make the request to, i.e https://fullnode.aptoslabs.devnet.com/v1
 * @param method - the request method "GET" | "POST"
 * @param endpoint (optional) - the endpoint to make the request to, i.e transactions
 * @param body (optional) - the body of the request
 * @param contentType (optional) - the content type to set the `content-type` header to,
 * by default is set to `application/json`
 * @param params (optional) - query params to add to the request
 * @param originMethod (optional) - the local method the request came from
 * @param overrides (optional) - a `ClientConfig` object type to override request data
 */
export type AptosRequest = {
  url: string;
  method: "GET" | "POST";
  endpoint?: string;
  body?: any;
  contentType?: string;
  params?: Record<string, string | AnyNumber | boolean | undefined>;
  originMethod?: string;
  overrides?: ClientConfig;
};

/**
 * The API response type
 *
 * @param status - the response status. i.e 200
 * @param statusText - the response message
 * @param data the response data
 * @param url the url the request was made to
 * @param headers the response headers
 * @param config (optional) - the request object
 * @param request (optional) - the request object
 */
export interface AptosResponse<Req, Res> {
  status: number;
  statusText: string;
  data: Res;
  url: string;
  headers: any;
  config?: any;
  request?: Req;
}

/**
 * The type returned from an API error
 *
 * @param name - the error name "AptosApiError"
 * @param url the url the request was made to
 * @param status - the response status. i.e 400
 * @param statusText - the response message
 * @param data the response data
 * @param request - the AptosRequest
 */
export class AptosApiError extends Error {
  readonly url: string;

  readonly status: number;

  readonly statusText: string;

  readonly data: any;

  readonly request: AptosRequest;

  constructor(request: AptosRequest, response: AptosResponse<any, any>, message: string) {
    super(message);

    this.name = "AptosApiError";
    this.url = response.url;
    this.status = response.status;
    this.statusText = response.statusText;
    this.data = response.data;
    this.request = request;
  }
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly message: string,
    public readonly errorCode?: string,
    public readonly vmErrorCode?: string,
  ) {
    super(message);
  }
}

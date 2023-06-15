import { ClientConfig } from "../types";
import { DEFAULT_NETWORK } from "../utils";

export class AptosConfig {
  readonly network: string;

  readonly clientConfig?: ClientConfig;

  constructor(config?: AptosConfig) {
    this.network = config?.network ?? DEFAULT_NETWORK;
    this.clientConfig = config?.clientConfig ?? {};
  }
}

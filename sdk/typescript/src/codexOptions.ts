export type SolaiAgentConfigValue = string | number | boolean | SolaiAgentConfigValue[] | SolaiAgentConfigObject;

export type SolaiAgentConfigObject = { [key: string]: SolaiAgentConfigValue };

export type SolaiAgentOptions = {
  codexPathOverride?: string;
  baseUrl?: string;
  apiKey?: string;
  /**
   * Additional `--config key=value` overrides to pass to the SolaiAgent.
   *
   * Provide a JSON object and the SDK will flatten it into dotted paths and
   * serialize values as TOML literals so they are compatible with the CLI's
   * `--config` parsing.
   */
  config?: SolaiAgentConfigObject;
  /**
   * Environment variables passed to the SolaiAgent process. When provided, the SDK
   * will not inherit variables from `process.env`.
   */
  env?: Record<string, string>;
};

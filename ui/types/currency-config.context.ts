import { Currency } from "./currency";

export const CURRENCY_NETWORKS = ["ic", "btc", "eth", "sol"] as const;
export type CurrencyNetwork = (typeof CURRENCY_NETWORKS)[number];

export type CurrencyConfigContextType = {
  enabledNetworks: CurrencyNetwork[];
  selectedCurrency: Currency;
  setSelectedCurrency(currency: Currency): void;
  isBTC: boolean;
  isSOL: boolean;
};

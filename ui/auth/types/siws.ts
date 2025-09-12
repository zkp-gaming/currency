import { WalletName } from "@solana/wallet-adapter-base";

import { AuthDataProvider } from "./auth-data";

export type AuthDataSiws = AuthDataProvider<
  "siws",
  { type: "siws"; provider: WalletName }
>;

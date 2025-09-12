import { SiwsIdentityProvider, useSiws } from 'ic-siws-js/react';
import { memo, PropsWithChildren, useMemo } from 'react';

import { Principal } from '@dfinity/principal';
import { Adapter, WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { ConnectionProvider, useWallet, WalletProvider } from '@solana/wallet-adapter-react';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-wallets';
import { clusterApiUrl } from '@solana/web3.js';
import { useQuery } from '@tanstack/react-query';
import { IC_HOST } from '@zk-game-dao/ui';

type Props = PropsWithChildren<{
  siwsProvidedCanisterId: Principal;
}>;

const AutoLoginProvider = memo<PropsWithChildren<PropsWithChildren>>(({ children }) => {
  const { login, identity } = useSiws();
  const { wallet } = useWallet();

  useQuery({
    queryKey: [
      "perform-login-siws",
      identity?.getPrincipal().toText(),
      wallet?.adapter.name,
    ],
    queryFn: async () => {
      if (!wallet || identity) return;
      await login();
      return true;
    },
    enabled: !!wallet && !identity,
    staleTime: Infinity,
    retry: false,
  });

  return children;
});

const ProvideSiws = memo<Props>(({ children, siwsProvidedCanisterId }) => {
  // Listen for changes to the selected wallet
  const { wallet } = useWallet();

  // Update the SiwsIdentityProvider with the selected wallet adapter
  return (
    <SiwsIdentityProvider
      canisterId={siwsProvidedCanisterId.toText()}
      adapter={wallet?.adapter}
      httpAgentOptions={{
        host: IC_HOST
      }}
    >
      <AutoLoginProvider>
        {children}
      </AutoLoginProvider>
    </SiwsIdentityProvider>
  );
});
ProvideSiws.displayName = "ProvideSiws";

export const ProvideSolanaLogins = memo<Props>((props) => {
  const network = WalletAdapterNetwork.Mainnet;
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);
  const wallets = useMemo((): Adapter[] => [new PhantomWalletAdapter()], []);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        <ProvideSiws {...props} />
      </WalletProvider>
    </ConnectionProvider>
  );
});
ProvideSolanaLogins.displayName = "ProvideSolanaLogins";

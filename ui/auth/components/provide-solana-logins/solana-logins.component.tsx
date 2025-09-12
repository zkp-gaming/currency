import classNames from 'classnames';
import { useSiws } from 'ic-siws-js/react';
import { memo, useMemo } from 'react';

import { useWallet, Wallet } from '@solana/wallet-adapter-react';
import { WalletIcon } from '@solana/wallet-adapter-react-ui';
import {
  ButtonComponent, ErrorComponent, Interactable, List, ListItem, LoadingAnimationComponent, useMutation
} from '@zk-game-dao/ui';

import { useIsSOL } from '../../../context';

export const SolanaLoginsComponent = memo<{
  onSuccess(): void;
}>(({ onSuccess }) => {
  const { wallets: walletsFromSystem, wallet: currentWallet, disconnect } = useWallet();
  const { login, identity, clear, prepareLogin, setAdapter } = useSiws();
  const isSOL = useIsSOL();
  // Some wallets don't work with siws yet
  const wallets = useMemo(() => walletsFromSystem.filter(
    ({ adapter }) => "signIn" in adapter && typeof adapter.signIn === "function"
  ), [walletsFromSystem]);
  const hasSomeConnectedWallets = useMemo(
    () => wallets.some((wallet) => wallet.adapter.connected),
    [wallets],
  );

  const loginToSolanaMut = useMutation({
    mutationFn: async (wallet: Wallet) => {
      if (!wallet)
        throw new Error("No wallet selected");

      await wallet.adapter.connect();

      await setAdapter(wallet.adapter);
      await login();
    },
    onSuccess: onSuccess,
  })

  if (!isSOL) return null;

  return (
    <>
      <style>
        {`
          .solflare-metamask-wallet-adapter-iframe iframe {
            z-index: 99999 !important;
          }
        `}
      </style>

      {!identity && currentWallet && (
        <Interactable>
          <ButtonComponent onClick={login}>
            Continue log in with {currentWallet.adapter.name}
          </ButtonComponent>
        </Interactable>
      )}

      {identity && (
        <ButtonComponent onClick={() => {
          clear();
          disconnect();
        }}>
          Log out
        </ButtonComponent>
      )}

      <ErrorComponent error={loginToSolanaMut.error} />

      <List label="Connect Wallet">
        {wallets.map((wallet) => (
          <ListItem
            key={wallet.adapter.name + wallet.adapter.icon}
            onClick={() => loginToSolanaMut.mutateAsync(wallet)}
            icon={(
              <div className={classNames("w-6 ml-5 mr-4 relative", {
                "opacity-50 pointer-events-none": wallet.readyState !== "Installed",
              })}>
                {wallet.adapter.connected && <span className='bg-green-500 absolute top-0 -translate-1/3 left-0 size-2 rounded-full' />}
                {wallet.readyState !== "Installed" && <span className='bg-red-500 absolute top-0 -translate-1/3 left-0 size-2 rounded-full' />}
                <WalletIcon
                  wallet={wallet}
                  className={classNames({
                    "opacity-80 group-hover:opacity-100 transition-opacity": !wallet.adapter.connected && hasSomeConnectedWallets,
                  })}
                />
              </div>
            )}
          >
            {(
              loginToSolanaMut.variables?.adapter.name === wallet.adapter.name &&
              loginToSolanaMut.isPending
            ) ? (
              <LoadingAnimationComponent variant="shimmer" className='ml-1'>Connecting to {wallet.adapter.name}</LoadingAnimationComponent>
            ) : (
              <span>{wallet.adapter.name}</span>
            )}

            {wallet.readyState !== "Installed" && (
              <span className="text-red-500 ml-2">Not installed</span>
            )}
          </ListItem>
        ))}
      </List>
    </>
  );
});

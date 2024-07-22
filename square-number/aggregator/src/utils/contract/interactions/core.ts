import { vaultAbi } from '../abis/vaultAbi';
import { client, coreContract, dssContract } from '../contract';

export async function getOperatorStakeMapping(
  operators: string[],
  minAcceptableStake: bigint
): Promise<[Map<string, bigint>, bigint]> {
  const stakeMapping: Map<string, bigint> = new Map();
  let totalStake: bigint = 0n;

  for (const operator of operators) {
    const stake = await getOperatorStakeNormalizedETH(operator);
    if (stake > minAcceptableStake) {
      stakeMapping.set(operator, stake);
      totalStake += stake;
    }
  }
  return [stakeMapping, totalStake];
}

export async function getOperatorStakeNormalizedETH(operator: string): Promise<bigint> {
  let stake = 0n;
  const vaults = (await coreContract.read.fetchVaultsStakedInDSS([operator, dssContract.address])) as string[];
  for (const vault of vaults) {
    //TODO: normalize all tokens to ETH
    stake += (await client.readContract({
      address: vault as `0x${string}`,
      abi: vaultAbi,
      functionName: 'totalAssets',
    })) as bigint;
  }
  return stake;
}

import { Address, createTestClient, getContract, http, publicActions, walletActions } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { foundry } from "viem/chains";

import { core as coreAddress, squareNumberDSS as dssAddress } from "@/../../contracts/contract-addresses.json";
import { env } from "@/config";

import { coreAbi } from "./abis/coreAbi";
import { squareNumberDssAbi } from "./abis/squareNumberDSSAbi";

// TODO change client based on env.NODE_ENV
export const client = createTestClient({
	account: privateKeyToAccount(env.PRIVATE_KEY as `0x${string}`),
	chain: foundry,
	mode: "anvil",
	transport: http(env.RPC_URL),
})
	.extend(publicActions)
	.extend(walletActions);

export const dssContractAddress = dssAddress;

export const dssContract = getContract({
	address: dssAddress as Address,
	abi: squareNumberDssAbi,
	client: client,
});

export const coreContract = getContract({
	address: coreAddress as Address,
	abi: coreAbi,
	client: client,
});

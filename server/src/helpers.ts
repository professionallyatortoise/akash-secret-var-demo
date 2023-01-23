import axios from "axios";
import { join } from "path";

const ROOT_DIR = join(__dirname, "..");

export async function queryData(): Promise<{
  price: number;
  redeemRate: number;
  borrowRate: number;
}> {
  const promises = [
    queryStAtomAtomPrice(),
    queryStAtomRedemptionRate(),
    queryAtomUmeeHedge(),
  ];
  const [price, redeemRate, borrowRate] = await Promise.all(promises);
  return {
    price,
    redeemRate,
    borrowRate,
  };
}

export function calcArb(redeemRate: number, price: number): number {
  return redeemRate / price;
}

export function calcAnnualizedArb(arb: number, DAYS_TO_REDEEM: number): number {
  return (arb ** (365 / DAYS_TO_REDEEM) - 1) * 100;
}

export async function calcNetBorrowRate(
  borrowRate: number,
  collateralRatio: number
): Promise<number> {
  const USDC_SUPPLY_RATE = await queryUsdcSupplyRate();

  return borrowRate * collateralRatio - USDC_SUPPLY_RATE;
}

export function calcNetDeltaNeutral(
  annualizedArb: number,
  collateralRatio: number,
  netBorrowRate: number
): number {
  return annualizedArb * collateralRatio - netBorrowRate;
}

export async function queryAtomUmeeHedge(): Promise<number> {
  return axios
    .get(
      "https://testnet-client-bff-ocstrhuppq-uc.a.run.app/convexity/assets/all"
    )
    .then((res: any) => {
      // find atom
      const [atom] = res.data.filter((x: any) => x.asset === "ATOM");
      const borrowRate = parseFloat(atom.borrow_apy);

      return borrowRate;
    });
}

async function queryUsdcSupplyRate(): Promise<number> {
  return axios
    .get(
      "https://testnet-client-bff-ocstrhuppq-uc.a.run.app/convexity/assets/all"
    )
    .then((res) => {
      // find usdc
      const [usdc] = res.data.filter((x: any) => x.asset === "USDC");
      const supplyRate = parseFloat(usdc.supply_apy);

      return supplyRate;
    });
}

async function queryStAtomRedemptionRate(): Promise<number> {
  return axios
    .get(
      "https://stride-api.polkachu.com/Stride-Labs/stride/stakeibc/host_zone"
    )
    .then((res) => {
      const rate = parseFloat(res.data.host_zone[0].redemption_rate);
      return rate;
    });
}
async function queryStAtomAtomPrice(): Promise<number> {
  return axios
    .get("https://api-osmosis.imperator.co/pools/v2/803")
    .then((res) => {
      const data = res.data;
      const price = data[0].amount / data[1].amount;

      return price;
    });
}

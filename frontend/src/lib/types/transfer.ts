import { ZScamInfo, type ScamInfo } from "./subreddit";
import * as z from "zod";

export const ZTransferScamInfo = ZScamInfo.omit({
  id: true,
});

export type TransferScamInfo = z.infer<typeof ZTransferScamInfo>;

const KEY = "$type";
const VALUE = "mlapibot-scam-transfer";

type Transfer = {
  [KEY]: typeof VALUE;
  scams: TransferScamInfo[];
};

export function initiateTransfer(scams: ScamInfo[]): string {
  const item: Transfer = {
    [KEY]: VALUE,
    scams: scams.map(({ id: _, ...rest }) => rest),
  };

  return btoa(JSON.stringify(item));
}

export function completeTransfer(transfer: string): TransferScamInfo[] {
  const data = JSON.parse(atob(transfer));

  if (typeof data === "object" && KEY in data && data[KEY] === VALUE) {
    data satisfies Transfer;

    return data.scams;
  } else {
    throw "invalid json";
  }
}

import { preferredIp } from "./diagnostics";
import type { CommandResponse, ProvisionResultView } from "../types";

export function provisionResultView({
  passwordlessSsid,
  provisionRefresh,
  provisionResponse,
  statusResponse,
}: {
  passwordlessSsid: string;
  provisionRefresh?: ProvisionResultView["statusRefresh"];
  provisionResponse?: CommandResponse;
  statusResponse?: CommandResponse;
}): ProvisionResultView | undefined {
  if (!provisionResponse) return undefined;
  return {
    ok: provisionResponse.ok,
    code: provisionResponse.code,
    text: provisionResponse.text,
    ssid: typeof provisionResponse.data?.ssid === "string" ? provisionResponse.data.ssid : passwordlessSsid,
    ip: typeof provisionResponse.data?.ip === "string" ? provisionResponse.data.ip : preferredIp(statusResponse),
    statusRefresh: provisionRefresh,
  };
}

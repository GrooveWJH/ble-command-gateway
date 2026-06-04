import type {
  BasicInfoRefreshFailure,
  BasicInfoRefreshState,
  CommandResponse,
} from "../../types";
import type { TFunction } from "../../i18n/I18nProvider";

export function basicInfoStateFromResponses(
  status?: CommandResponse,
  capabilities?: CommandResponse,
  t?: TFunction,
): BasicInfoRefreshState {
  const failures: BasicInfoRefreshFailure[] = [];
  if (!status?.ok) {
    failures.push({
      command: "system.status",
      message: status?.text || status?.code || t?.("basic.statusFailed") || "System status read failed",
    });
  }
  if (!isUsableCapabilitiesResponse(capabilities)) {
    failures.push({
      command: "system.capabilities",
      message: capabilities?.text || capabilities?.code || t?.("basic.capabilitiesFailed") || "Protocol capabilities read failed",
    });
  }
  if (failures.length === 0) {
    return { state: "fresh" };
  }
  return failures.length === 2
    ? { state: "failed", failures }
    : { state: "partial", failures };
}

export function isUsableCapabilitiesResponse(response?: CommandResponse): boolean {
  if (!response) return false;
  if (response.ok) return true;
  const data = response.data;
  if (!data) return false;
  return typeof data.protocol_version === "string"
    || typeof data.payload_limit === "number"
    || Array.isArray(data.commands)
    || Array.isArray(data.features);
}

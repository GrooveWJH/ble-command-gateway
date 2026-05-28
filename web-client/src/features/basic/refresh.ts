import type {
  BasicInfoRefreshFailure,
  BasicInfoRefreshState,
  CommandResponse,
} from "../../types";
import { capabilitiesView } from "../../ui/diagnostics";

export function basicInfoStateFromResponses(
  status?: CommandResponse,
  capabilities?: CommandResponse,
): BasicInfoRefreshState {
  const failures: BasicInfoRefreshFailure[] = [];
  if (!status?.ok) {
    failures.push({
      command: "system.status",
      message: status?.text || status?.code || "读取系统状态失败",
    });
  }
  if (!isUsableCapabilitiesResponse(capabilities)) {
    failures.push({
      command: "system.capabilities",
      message: capabilities?.text || capabilities?.code || "读取协议能力失败",
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
  const capabilities = capabilitiesView(response);
  if (!capabilities) return false;
  return capabilities.protocolVersion !== "未知"
    || capabilities.payloadLimit !== "未知"
    || capabilities.commands.length > 0
    || capabilities.features.length > 0;
}

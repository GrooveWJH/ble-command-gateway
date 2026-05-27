import { ProgressIndicator, ProgressStep } from "@carbon/react";

import { JOURNEY_STEPS, journeyIndex } from "../ui/journey";
import type { ProvisionJourneyStep } from "../types";

export function JourneyRail({ current }: { current: ProvisionJourneyStep }) {
  return (
    <aside className="journey-rail" aria-label="配网步骤">
      <h2>用户旅程</h2>
      <ProgressIndicator currentIndex={journeyIndex(current)} vertical>
        {JOURNEY_STEPS.map((step) => (
          <ProgressStep
            key={step.id}
            label={step.label}
            description={stepDescription(step.id)}
          />
        ))}
      </ProgressIndicator>
    </aside>
  );
}

function stepDescription(step: ProvisionJourneyStep): string {
  switch (step) {
    case "environment":
      return "浏览器与 HTTPS";
    case "connect":
      return "选择 yundrone-*";
    case "scan":
      return "读取周边热点";
    case "select":
      return "选择或手动输入";
    case "credentials":
      return "密码不落盘";
    case "provision":
      return "等待设备回执";
    case "result":
      return "确认成功或恢复";
  }
}

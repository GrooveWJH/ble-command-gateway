import { ProgressIndicator, ProgressStep } from "@carbon/react";

import { JOURNEY_STEPS, journeyIndex } from "../ui/journey";
import type { ProvisionJourneyStep } from "../types";

export function ProvisionSteps({ current }: { current: ProvisionJourneyStep }) {
  return (
    <section className="yd-provision-steps" aria-label="配网步骤">
      <div>
        <h3>配网步骤</h3>
        <p>把连接、扫描、选择网络和结果确认压缩在当前工作区内。</p>
      </div>
      <ProgressIndicator currentIndex={journeyIndex(current)} spaceEqually>
        {JOURNEY_STEPS.map((step) => (
          <ProgressStep
            key={step.id}
            label={step.label}
            description={stepDescription(step.id)}
          />
        ))}
      </ProgressIndicator>
    </section>
  );
}

function stepDescription(step: ProvisionJourneyStep): string {
  switch (step) {
    case "environment":
      return "浏览器与 HTTPS";
    case "connect":
      return "选择 yundrone-*";
    case "scan":
      return "读取热点";
    case "select":
      return "选择或输入";
    case "credentials":
      return "密码不落盘";
    case "provision":
      return "等待回执";
    case "result":
      return "确认 IP";
  }
}

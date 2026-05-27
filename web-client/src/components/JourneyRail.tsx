import { JOURNEY_STEPS, journeyIndex } from "../ui/journey";
import type { ProvisionJourneyStep } from "../types";

export function JourneyRail({ current }: { current: ProvisionJourneyStep }) {
  return (
    <aside className="journey-rail" aria-label="配网步骤">
      <h2>用户旅程</h2>
      <ol className="journey-steps">
        {JOURNEY_STEPS.map((step) => (
          <li
            key={step.id}
            className={stepClass(journeyIndex(current), journeyIndex(step.id))}
          >
            <span className="journey-steps__marker" />
            <div>
              <strong>{step.label}</strong>
              <span>{stepDescription(step.id)}</span>
            </div>
          </li>
        ))}
      </ol>
    </aside>
  );
}

function stepClass(currentIndex: number, stepIndex: number): string {
  if (stepIndex < currentIndex) return "journey-steps__item journey-steps__item--done";
  if (stepIndex === currentIndex) return "journey-steps__item journey-steps__item--current";
  return "journey-steps__item";
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

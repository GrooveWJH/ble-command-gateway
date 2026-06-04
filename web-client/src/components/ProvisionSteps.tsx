import { ProgressIndicator, ProgressStep } from "@carbon/react";

import { useI18n } from "../i18n/useI18n";
import { JOURNEY_STEPS, journeyIndex } from "../ui/journey";
import type { ProvisionJourneyStep } from "../types";

export function ProvisionSteps({ current }: { current: ProvisionJourneyStep }) {
  const { t } = useI18n();
  return (
    <section className="yd-provision-steps" aria-label={t("steps.aria")}>
      <div>
        <h3>{t("steps.title")}</h3>
        <p>{t("steps.subtitle")}</p>
      </div>
      <ProgressIndicator currentIndex={journeyIndex(current)} spaceEqually>
        {JOURNEY_STEPS.map((step) => (
          <ProgressStep
            key={step.id}
            label={t(step.labelKey)}
            description={stepDescription(step.id, t)}
          />
        ))}
      </ProgressIndicator>
    </section>
  );
}

function stepDescription(step: ProvisionJourneyStep, t: ReturnType<typeof useI18n>["t"]): string {
  switch (step) {
    case "environment":
      return t("steps.environment");
    case "connect":
      return t("steps.connect");
    case "scan":
      return t("steps.scan");
    case "select":
      return t("steps.select");
    case "credentials":
      return t("steps.credentials");
    case "provision":
      return t("steps.provision");
    case "result":
      return t("steps.result");
  }
}

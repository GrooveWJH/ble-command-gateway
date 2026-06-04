import { CopyButton, Modal, Tag } from "@carbon/react";
import { Terminal } from "@carbon/icons-react";
import { useState } from "react";

import { useI18n } from "../i18n/useI18n";

export const INSTALL_COMMAND = "bash <(curl -fsSL https://install.yundrone.cn/ble-wifi-tool.sh)";

export function InstallScriptCallout() {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const copyCommand = () => {
    void navigator.clipboard?.writeText(INSTALL_COMMAND);
  };
  return (
    <>
      <button className="yd-install-badge" type="button" onClick={() => setOpen(true)}>
        <Terminal size={16} aria-hidden="true" />
        <span>{t("header.installTool")}</span>
      </button>
      <Modal
        className="yd-install-modal"
        modalHeading={t("header.installTool")}
        modalLabel={t("install.modalLabel")}
        open={open}
        passiveModal
        size="md"
        onRequestClose={() => setOpen(false)}
        closeButtonLabel={t("install.close")}
      >
        <p>{t("install.description")}</p>
        <div className="yd-install-command" aria-label={t("install.commandAria")}>
          <code>{INSTALL_COMMAND}</code>
          <CopyButton
            align="bottom-right"
            feedback={t("install.copied")}
            iconDescription={t("install.copy")}
            onClick={copyCommand}
          />
        </div>
        <div className="yd-install-details">
          <Tag type="blue">{t("install.tui")}</Tag>
          <Tag type="green">{t("install.linux")}</Tag>
          <Tag type="cyan">{t("install.macos")}</Tag>
        </div>
      </Modal>
    </>
  );
}

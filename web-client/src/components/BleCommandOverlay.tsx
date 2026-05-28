import { InlineLoading } from "@carbon/react";

export function BleCommandOverlay({ active, label }: { active: boolean; label: string }) {
  if (!active) return null;
  return (
    <div className="yd-ble-overlay" data-testid="ble-command-overlay">
      <div
        aria-busy="true"
        aria-label="BLE 操作进行中"
        aria-live="polite"
        className="yd-ble-overlay__card"
        role="status"
      >
        <InlineLoading
          description={label}
          iconDescription="正在处理"
          status="active"
        />
        <p>请保持设备靠近并避免切换页面，操作完成后界面会自动更新。</p>
      </div>
    </div>
  );
}

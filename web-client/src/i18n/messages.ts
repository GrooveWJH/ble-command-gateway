import { enMessages } from "./messages.en";
import { zhMessages } from "./messages.zh";

export type MessageKey = keyof typeof enMessages;
export type Messages = Record<MessageKey, string>;

export const messages = {
  en: enMessages,
  zh: zhMessages,
} as const;

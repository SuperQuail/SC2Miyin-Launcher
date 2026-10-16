import { useCallback, useState } from "react";

/** 忙碌锁：同一时间只允许一个会写盘的动作（原来的 `busy` 就是干这个的）。 */
export interface BusyApi {
  busy: boolean;
  /** 跑一个动作，前后自动开关忙碌锁。 */
  run: <T>(action: () => Promise<T>) => Promise<T>;
}

export function useBusy(): BusyApi {
  const [busy, setBusy] = useState(false);

  const run = useCallback(async <T>(action: () => Promise<T>): Promise<T> => {
    setBusy(true);
    try {
      return await action();
    } finally {
      setBusy(false);
    }
  }, []);

  return { busy, run };
}

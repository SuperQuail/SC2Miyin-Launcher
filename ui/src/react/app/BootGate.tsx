import { useEffect, useState, type ReactNode } from "react";

import { errorText } from "../../api/errors";

import { waitForBoot } from "./boot";

/**
 * 等后端把数据准备好，再挂外壳。
 *
 * 数据目录迁移 / 启动时的合并已经挪到后台线程（见 src-tauri/src/lib.rs 的 run）——
 * 在它们完成之前**任何**碰 AppState 的命令都会让后端 panic，所以外壳必须等这一步。
 * 平时是毫秒级（没有旧数据要搬）；只有第一次迁移大库时才会停一会儿，那时这页就是
 * 用户能看到的「正在准备数据」。
 */
export function BootGate({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false);
  const [failure, setFailure] = useState("");

  useEffect(() => {
    let live = true;
    waitForBoot().then(
      () => {
        if (live) setReady(true);
      },
      (error: unknown) => {
        if (live) setFailure(errorText(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  if (ready) return <>{children}</>;

  if (failure) {
    return (
      <div className="boot-fail">
        <div className="boot-fail__card">
          <h1 className="boot-fail__title">启动失败</h1>
          <p className="boot-fail__text">后台初始化未完成。可点击下方「重新加载」重试。</p>
          <pre className="boot-fail__reason">{failure}</pre>
          <button type="button" className="btn btn-primary" onClick={() => location.reload()}>
            重新加载
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="boot">
      <span className="boot__spinner" />
      <p className="boot__title">正在准备数据…</p>
      <p className="boot__hint">首次启动需要迁移旧数据；战役库较大时耗时较长</p>
    </div>
  );
}

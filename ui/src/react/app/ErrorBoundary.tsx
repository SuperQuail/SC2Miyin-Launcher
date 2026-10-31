import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * 顶层错误边界。
 *
 * React 18 里任何一处**渲染期**异常都会把整棵树卸载掉 —— 没有边界的话，用户看到的
 * 就是一片白，而且没有恢复的路（浏览器刷新键被 installWebviewGuards 拦掉了）。
 * 这里把它换成一句人话 + 原始错误 + 「重新加载」。
 *
 * 只兜渲染期的错：事件回调与异步里的错走 startup.ts 的 window 监听。
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // 控制台那份是给开发者看的；用户那页刻意不带 componentStack
    console.error("[miyin] 界面崩溃", error, info.componentStack);
  }

  render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="boot-fail">
        <div className="boot-fail__card">
          <h1 className="boot-fail__title">界面发生错误</h1>
          <p className="boot-fail__text">
            可点击下方「重新加载」重试；若问题仍然存在，请将错误信息反馈给开发者。
          </p>
          <pre className="boot-fail__reason">{error.stack || error.message}</pre>
          <button type="button" className="btn btn-primary" onClick={() => location.reload()}>
            重新加载
          </button>
        </div>
      </div>
    );
  }
}

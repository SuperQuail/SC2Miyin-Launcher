/**
 * 复制文本。
 *
 * 原来是**两套实现**：一处在作弊码页、一处在战役页，各自都带 Clipboard API + execCommand
 * 回退。WebView 里哪条路通不一定，所以两条都留着 —— 但只留这一份。
 *
 * 返回是否成功（失败不抛，调用方决定怎么提示）。
 */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // 退回老办法：造一个看不见的输入框，选中后复制
    try {
      const holder = document.createElement("textarea");
      holder.value = text;
      holder.setAttribute("readonly", "");
      holder.style.position = "fixed";
      holder.style.opacity = "0";
      document.body.appendChild(holder);
      holder.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(holder);
      return ok;
    } catch {
      return false;
    }
  }
}

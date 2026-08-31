// 全局命令面板（Ctrl+K）开关状态；App 与 CommandPalette 共享。
let _open = $state(false);

export const paletteStore = {
  get open() { return _open; },
  setOpen(value: boolean) { _open = value; },
  close() { _open = false; },
  toggle() { _open = !_open; },
};

export function reachedEnd(
  scrollTop: number,
  clientHeight: number,
  scrollHeight: number,
  slack = 4,
): boolean {
  return scrollHeight <= clientHeight + slack || scrollTop + clientHeight >= scrollHeight - slack;
}

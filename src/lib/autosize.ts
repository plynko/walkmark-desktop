/** Fit a textarea after user input, programmatic edits, or a change in width. */
export function autosize(node: HTMLTextAreaElement, _value?: string) {
  let frame = 0;
  let width = -1;

  const measure = () => {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (node.hidden || node.getClientRects().length === 0) return;
      node.style.height = "auto";
      const border = node.offsetHeight - node.clientHeight;
      node.style.height = `${node.scrollHeight + border}px`;
    });
  };
  const observer = new ResizeObserver(([entry]) => {
    const nextWidth = entry.contentRect.width;
    if (width !== nextWidth) {
      width = nextWidth;
      measure();
    }
  });
  observer.observe(node);
  node.addEventListener("input", measure);
  measure();

  return {
    update: measure,
    destroy() {
      cancelAnimationFrame(frame);
      observer.disconnect();
      node.removeEventListener("input", measure);
    },
  };
}

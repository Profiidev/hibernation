import { Context } from 'runed';
import type { ReadableBoxedValues, WritableBoxedValues } from 'svelte-toolbelt';
import type { CodeRootProps } from './types';
import { highlighter } from './shiki';
import type { HighlighterCore } from 'shiki';

type CodeOverflowStateProps = WritableBoxedValues<{
  collapsed: boolean;
}>;

class CodeOverflowState {
  // oxlint-disable-next-line explicit-member-accessibility
  constructor(readonly opts: CodeOverflowStateProps) {
    this.toggleCollapsed = this.toggleCollapsed.bind(this);
  }

  // oxlint-disable-next-line explicit-member-accessibility
  toggleCollapsed() {
    this.opts.collapsed.current = !this.opts.collapsed.current;
  }

  // oxlint-disable-next-line explicit-member-accessibility
  get collapsed() {
    return this.opts.collapsed.current;
  }
}

type CodeRootStateProps = ReadableBoxedValues<{
  code: string;
  lang: NonNullable<CodeRootProps['lang']>;
  hideLines: boolean;
  highlight: CodeRootProps['highlight'];
}>;

class CodeRootState {
  // oxlint-disable-next-line explicit-member-accessibility
  highlighter: HighlighterCore | null = $state(null); // oxlint-disable-line no-null

  // oxlint-disable-next-line explicit-member-accessibility
  constructor(
    // oxlint-disable-next-line explicit-member-accessibility
    readonly opts: CodeRootStateProps,
    // oxlint-disable-next-line explicit-member-accessibility
    readonly overflow?: CodeOverflowState
  ) {
    const _ = highlighter.then((hl) => (this.highlighter = hl));
  }

  // oxlint-disable-next-line explicit-member-accessibility
  highlight(code: string) {
    return this.highlighter?.codeToHtml(code, {
      lang: this.opts.lang.current,
      themes: {
        dark: 'github-dark-default',
        light: 'github-light-default'
      },
      transformers: [
        {
          line: (node, line) => {
            if (within(line, this.opts.highlight.current)) {
              node.properties.class += ' line--highlighted';
            }

            return node;
          },
          pre: (el) => {
            el.properties.style = '';

            if (!this.opts.hideLines.current) {
              el.properties.class += ' line-numbers';
            }

            return el;
          }
        }
      ]
    });
  }

  // oxlint-disable-next-line explicit-member-accessibility
  get code() {
    return this.opts.code.current;
  }

  // oxlint-disable-next-line explicit-member-accessibility
  highlighted = $derived(this.highlight(this.code) ?? '');
}

const within = (num: number, range: CodeRootProps['highlight']) => {
  if (!range) {
    return false;
  }

  let isWithin = false;

  for (const r of range) {
    if (typeof r === 'number') {
      if (num === r) {
        isWithin = true;
        break;
      }
      continue;
    }

    if (r[0] <= num && num <= r[1]) {
      isWithin = true;
      break;
    }
  }

  return isWithin;
};

class CodeCopyButtonState {
  // oxlint-disable-next-line explicit-member-accessibility
  constructor(readonly root: CodeRootState) {}

  // oxlint-disable-next-line explicit-member-accessibility
  get code() {
    return this.root.opts.code.current;
  }
}

const overflowCtx = new Context<CodeOverflowState>('code-overflow-state');

const ctx = new Context<CodeRootState>('code-root-state');

export const useCodeOverflow = (props: CodeOverflowStateProps) =>
  overflowCtx.set(new CodeOverflowState(props));

export const useCode = (props: CodeRootStateProps) =>
  ctx.set(new CodeRootState(props, overflowCtx.getOr(undefined)));

export const useCodeCopyButton = () => new CodeCopyButtonState(ctx.get());

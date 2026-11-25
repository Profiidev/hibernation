export const linesToPatterns = (value: string): string[] =>
  value
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

export const patternsToLines = (patterns: string[]): string =>
  patterns.join('\n');

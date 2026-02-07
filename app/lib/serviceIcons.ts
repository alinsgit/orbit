export function getServiceIcon(type: string): string {
  const t = type.toLowerCase()
  if (t === 'nginx') return '\u{1F310}'
  if (t === 'apache') return '\u{1FAB6}'
  if (t === 'php') return '\u{1F418}'
  if (t === 'mariadb') return '\u{1F5C4}\uFE0F'
  if (t === 'nodejs') return '\u{1F49A}'
  if (t === 'python') return '\u{1F40D}'
  if (t === 'bun') return '\u{1F95F}'
  return '\u2699\uFE0F'
}

---
paths:
  - "**/*.ts"
  - "**/*.tsx"
---

# TypeScript Kuralları

## Genel
- Strict mode aktif olmalı
- `any` kullanımından kaçın, `unknown` tercih et
- Interface'leri `I` prefix'i olmadan yaz: `User` not `IUser`

## React/TSX
- Functional components kullan
- Props için interface tanımla
- Custom hooks `use` prefix'i ile başlamalı

## Import Sıralaması
1. React/Next.js
2. Third-party libraries
3. Internal modules
4. Types
5. Styles

## Örnek Component
```tsx
interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'secondary';
}

export function Button({ label, onClick, variant = 'primary' }: ButtonProps) {
  return (
    <button className={`btn btn-${variant}`} onClick={onClick}>
      {label}
    </button>
  );
}
```

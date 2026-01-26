# Kod Konvansiyonları

## Dosya İsimlendirme
- **Klasörler**: `kebab-case` → `user-management/`
- **Componentler**: `PascalCase` → `UserCard.tsx`
- **Utilities**: `camelCase` → `formatDate.ts`
- **Sabitler**: `SCREAMING_SNAKE` → `API_ENDPOINTS.ts`

## Değişken İsimlendirme
```typescript
// Değişkenler: camelCase
const userName = 'John';
const isActive = true;

// Sabitler: SCREAMING_SNAKE_CASE
const MAX_RETRY_COUNT = 3;
const API_BASE_URL = 'https://api.example.com';

// Fonksiyonlar: camelCase, fiil ile başla
function getUserById(id: string) {}
function calculateTotal(items: Item[]) {}

// Boolean: is/has/can/should prefix
const isLoading = true;
const hasPermission = false;
const canEdit = true;
```

## Commit Mesajları
```
feat: Yeni özellik eklendi
fix: Bug düzeltildi
docs: Dokümantasyon güncellendi
style: Formatting değişikliği
refactor: Kod refactoring
test: Test eklendi/güncellendi
chore: Build/config değişikliği
```

## Branch İsimlendirme
```
feature/add-user-authentication
fix/cart-calculation-error
hotfix/security-patch
release/v1.2.0
```

## Proje Yapısı (Önerilen)
```
src/
├── components/      # UI bileşenleri
│   ├── common/      # Paylaşılan (Button, Input)
│   └── features/    # Feature-specific
├── hooks/           # Custom React hooks
├── utils/           # Yardımcı fonksiyonlar
├── services/        # API calls
├── types/           # TypeScript types
├── styles/          # Global styles
└── pages/           # Route pages (Next.js)
```

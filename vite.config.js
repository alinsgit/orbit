import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
// https://vitejs.dev/config/
export default defineConfig({
    plugins: [
        react(),
        tailwindcss(),
    ],
    server: {
        watch: {
            ignored: ['**/core/target/**'],
        },
    },
    build: {
        rollupOptions: {
            output: {
                manualChunks(id) {
                    if (id.includes('node_modules/react-dom') || id.includes('node_modules/react/')) return 'vendor';
                    if (id.includes('node_modules/xterm')) return 'terminal';
                    if (id.includes('node_modules/lucide-react')) return 'icons';
                },
            },
        },
    },
});

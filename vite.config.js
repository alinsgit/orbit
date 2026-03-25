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
                manualChunks: {
                    vendor: ['react', 'react-dom'],
                    terminal: ['xterm', 'xterm-addon-fit', 'xterm-addon-unicode11', 'xterm-addon-web-links'],
                    icons: ['lucide-react'],
                },
            },
        },
    },
});

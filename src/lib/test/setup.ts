// Vitest setup file
import { vi } from 'vitest';
import '@testing-library/svelte/vitest';

// Mock Tauri API for tests
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(() => Promise.resolve(() => {})),
	emit: vi.fn(),
}));

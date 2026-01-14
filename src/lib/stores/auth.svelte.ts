// Auth store (01_auth.md)

import type { AuthStatus, SessionValidity, AuthIndicatorState } from '$lib/types';
import * as authApi from '$lib/tauri/auth';

function createAuthStore() {
  let status = $state<AuthStatus>({
    is_authenticated: false,
    has_saved_credentials: false,
    storage_type: 'secure',
    storage_error: null
  });
  let sessionValidity = $state<SessionValidity | null>(null);
  let isLoading = $state(false);
  let isCheckingSession = $state(false);
  let error = $state<string | null>(null);

  // Derived: indicator state based on all conditions
  let indicatorState = $derived.by((): AuthIndicatorState => {
    // Check for storage error first
    if (status.storage_error) {
      return 'storage_error';
    }

    // Not authenticated
    if (!status.is_authenticated) {
      return 'unauthenticated';
    }

    // Authenticated - check session state
    if (isCheckingSession) {
      return 'authenticated_checking';
    }

    if (sessionValidity) {
      if (sessionValidity.is_valid) {
        return 'authenticated_valid';
      }
      // Check if it was a network error
      if (sessionValidity.error?.includes('Network')) {
        return 'authenticated_error';
      }
      // Session is invalid
      return 'authenticated_invalid';
    }

    // Have credentials but haven't checked yet
    return 'authenticated_checking';
  });

  return {
    get status() {
      return status;
    },
    get isAuthenticated() {
      return status.is_authenticated;
    },
    get hasSavedCredentials() {
      return status.has_saved_credentials;
    },
    get storageType() {
      return status.storage_type;
    },
    get storageError() {
      return status.storage_error;
    },
    get sessionValidity() {
      return sessionValidity;
    },
    get indicatorState() {
      return indicatorState;
    },
    get isLoading() {
      return isLoading;
    },
    get isCheckingSession() {
      return isCheckingSession;
    },
    get error() {
      return error;
    },

    async refreshStatus() {
      isLoading = true;
      error = null;
      try {
        status = await authApi.authGetStatus();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        isLoading = false;
      }
    },

    async loadCredentials() {
      isLoading = true;
      error = null;
      try {
        await authApi.authLoadCredentials();
        await this.refreshStatus();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return false;
      } finally {
        isLoading = false;
      }
    },

    async saveRawCookies(rawCookies: string) {
      isLoading = true;
      error = null;
      try {
        await authApi.authSaveRawCookies(rawCookies);
        await this.refreshStatus();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        throw e;
      } finally {
        isLoading = false;
      }
    },

    async saveCredentials(
      sid: string,
      hsid: string,
      ssid: string,
      apisid: string,
      sapisid: string
    ) {
      isLoading = true;
      error = null;
      try {
        await authApi.authSaveCredentials(sid, hsid, ssid, apisid, sapisid);
        await this.refreshStatus();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        throw e;
      } finally {
        isLoading = false;
      }
    },

    async deleteCredentials() {
      isLoading = true;
      error = null;
      try {
        await authApi.authDeleteCredentials();
        sessionValidity = null;
        await this.refreshStatus();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      } finally {
        isLoading = false;
      }
    },

    async validate() {
      error = null;
      try {
        return await authApi.authValidateCredentials();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return false;
      }
    },

    async checkSessionValidity() {
      isCheckingSession = true;
      error = null;
      try {
        sessionValidity = await authApi.authCheckSessionValidity();
        return sessionValidity;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        sessionValidity = {
          is_valid: false,
          checked_at: new Date().toISOString(),
          error: error
        };
        return sessionValidity;
      } finally {
        isCheckingSession = false;
      }
    },

    async useFallbackStorage() {
      isLoading = true;
      error = null;
      try {
        await authApi.authUseFallbackStorage();
        await this.refreshStatus();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return false;
      } finally {
        isLoading = false;
      }
    },

    async openLoginWindow() {
      isLoading = true;
      error = null;
      try {
        await authApi.authOpenWindow();
        await this.refreshStatus();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        throw e;
      } finally {
        isLoading = false;
      }
    },

    clearError() {
      error = null;
    }
  };
}

export const authStore = createAuthStore();

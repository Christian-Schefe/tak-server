import { useJwtStore } from '@/features/auth';
import type z from 'zod';
import { jwtDecode } from 'jwt-decode';
import { useRefreshAccount } from '@/api/auth';

function isJwtCloseToExpiring(jwt: string): boolean {
  const decoded = jwtDecode(jwt);
  if (typeof decoded.exp !== 'number') {
    console.warn('JWT does not have a valid exp claim:', decoded);
    return true;
  }
  const currentTime = Math.floor(Date.now() / 1000);
  // Livetime of JWT is 24 hours, so consider it close to expiring if it expires within the next five hours
  return decoded.exp - currentTime <= 5 * 60 * 60;
}

export function useFetch() {
  const jwtStore = useJwtStore();
  const refreshAccount = useRefreshAccount();

  function scheduleTokenRefresh(jwt: string | null) {
    if (jwt === null) {
      return;
    }
    if (isJwtCloseToExpiring(jwt)) {
      console.log('JWT is close to expiring, refreshing account info...');
      void refreshAccount();
    }
  }

  async function fetch2(
    path: string,
    method: 'GET' | 'POST' | 'DELETE' = 'GET',
    body?: unknown,
    auth: boolean = true,
    preventRefresh: boolean = false,
  ): Promise<Response> {
    const headers: HeadersInit = {};
    if (auth && jwtStore.jwt !== null) {
      if (!preventRefresh) {
        scheduleTokenRefresh(jwtStore.jwt);
      }
      headers['Authorization'] = `Bearer ${jwtStore.jwt}`;
    }
    if (method === 'POST' && body !== undefined && !(body instanceof FormData)) {
      headers['Content-Type'] = 'application/json';
    }
    let response: Response;
    if (method === 'POST') {
      response = await fetch(path, {
        method: 'POST',
        headers,
        body:
          body !== undefined ? (body instanceof FormData ? body : JSON.stringify(body)) : undefined,
      });
    } else {
      response = await fetch(path, {
        method,
        headers,
      });
    }
    if (!response.ok) {
      throw new Error(
        `Failed to fetch ${path}: ${response.status.toString()} ${response.statusText}`,
      );
    }
    return response;
  }

  async function fetchTyped<T>(
    schema: z.ZodType<T>,
    path: string,
    method: 'GET' | 'POST' | 'DELETE' = 'GET',
    body?: unknown,
    auth: boolean = true,
    preventRefresh: boolean = false,
  ): Promise<T> {
    const response = await fetch2(path, method, body, auth, preventRefresh);
    const data = await response.json();
    const parsed = schema.safeParse(data);
    if (!parsed.success) {
      console.error(`Failed to parse response from ${path}:`, parsed.error);
      throw new Error(`Failed to parse response from ${path}: ${parsed.error}`);
    }
    return parsed.data;
  }

  return {
    fetch: fetch2,
    fetchTyped: fetchTyped,
  };
}

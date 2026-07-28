import { accountQueryKey, useAccount } from '@/api/auth';
import {
  Configuration,
  FrontendApi,
  type ContinueWith,
  type LoginFlow,
  type RecoveryFlow,
  type RegistrationFlow,
  type Session,
  type SettingsFlow,
  type UpdateLoginFlowBody,
  type UpdateRecoveryFlowBody,
  type UpdateRegistrationFlowBody,
  type UpdateSettingsFlowBody,
  type UpdateVerificationFlowBody,
  type VerificationFlow,
} from '@ory/client';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { useStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { useRouter } from 'vue-router';
import z from 'zod';

const axiosError = z.object({
  response: z.object({
    status: z.number(),
  }),
});

const kratosSessionQueryKey = ['kratos-session'] as const;

export const useJwtStore = defineStore('auth-jwt', () => {
  const storedJWT = useStorage<string | null>('jwt', null);
  const jwt = ref<string | null>(storedJWT.value);

  function setJwt(newJwt: string | null) {
    jwt.value = newJwt;
    storedJWT.value = newJwt;
  }

  return { jwt, setJwt };
});

type AuthState = { type: 'logged_in'; session: Session } | { type: 'logged_out' | 'loading' };

export const useAuthStore = defineStore('auth', () => {
  const authState = ref<AuthState>({ type: 'loading' });

  const jwtStore = useJwtStore();
  const { data: account } = useAccount();
  watch(account, () => {
    if (account.value && account.value.jwt !== jwtStore.jwt) {
      jwtStore.setJwt(account.value.jwt);
    }
  });
  const {
    isFetching,
    isError,
    data: session,
    error,
  } = useQuery({
    queryKey: kratosSessionQueryKey,
    queryFn: async () => {
      try {
        return await kratos.toSession();
      } catch (error) {
        const parsedError = axiosError.safeParse(error);
        if (parsedError.success) {
          if (parsedError.data.response.status === 401) {
            // Not logged in, return null session
            return null;
          }
        }

        throw error;
      }
    },
  });

  const queryClient = useQueryClient();

  async function logout() {
    if (authState.value.type === 'logged_in') {
      const flow = await kratos.createBrowserLogoutFlow();
      await kratos.updateLogoutFlow({ token: flow.data.logout_token });
    }
    await queryClient.refetchQueries({ queryKey: kratosSessionQueryKey });
  }

  watch([isFetching, isError, session, error], () => {
    let newAuthState: AuthState;
    if (isFetching.value) {
      newAuthState = { type: 'loading' };
    } else if (isError.value) {
      console.error('Error fetching session:', error.value);
      newAuthState = { type: 'logged_out' };
    } else if (session.value) {
      newAuthState = { type: 'logged_in', session: session.value.data };
    } else {
      newAuthState = { type: 'logged_out' };
    }
    authState.value = newAuthState;
    if (newAuthState.type !== 'loading') {
      void queryClient.refetchQueries({ queryKey: accountQueryKey });
    }
  });

  return { authState, logout };
});

const kratosConfig = new Configuration({
  basePath: '/auth',
  baseOptions: {
    withCredentials: true,
  },
});

export const kratos = new FrontendApi(kratosConfig);

export type KratosFlowType = 'login' | 'registration' | 'verification' | 'settings' | 'recovery';
type KratosFlow = RecoveryFlow | VerificationFlow | SettingsFlow | LoginFlow | RegistrationFlow;

export function useKratosFlow(flowType: MaybeRefOrGetter<KratosFlowType>, initialFlowId?: string) {
  const flow = ref<KratosFlow | null>(null);

  const { data: flowData } = useQuery({
    queryKey: ['kratos-flow', flowType],
    queryFn: async () => {
      const id = initialFlowId;
      const type = toValue(flowType);
      if (id !== undefined) {
        if (type === 'login') {
          return await kratos.getLoginFlow({ id });
        } else if (type === 'registration') {
          return await kratos.getRegistrationFlow({ id });
        } else if (type === 'verification') {
          return await kratos.getVerificationFlow({ id });
        } else if (type === 'settings') {
          return await kratos.getSettingsFlow({ id });
        } else {
          return await kratos.getRecoveryFlow({ id });
        }
      } else {
        if (type === 'login') {
          return await kratos.createBrowserLoginFlow();
        } else if (type === 'registration') {
          return await kratos.createBrowserRegistrationFlow();
        } else if (type === 'verification') {
          return await kratos.createBrowserVerificationFlow();
        } else if (type === 'settings') {
          return await kratos.createBrowserSettingsFlow();
        } else {
          return await kratos.createBrowserRecoveryFlow();
        }
      }
    },
  });
  watch(flowData, (newFlowData) => {
    if (newFlowData) {
      flow.value = newFlowData.data;
    } else {
      flow.value = null;
    }
  });

  const queryClient = useQueryClient();
  const router = useRouter();

  async function reloadKratosSession() {
    await queryClient.refetchQueries({ queryKey: kratosSessionQueryKey });
  }

  async function handleContinueWith(continueWith: ContinueWith) {
    console.log('Handling continue_with action:', continueWith.action, continueWith);
    switch (continueWith.action) {
      case 'redirect_browser_to':
        await router.push(continueWith.redirect_browser_to);
        break;
      case 'show_recovery_ui':
        await router.push('/recover');
        break;
      case 'show_settings_ui':
        await router.push('/account');
        break;
      case 'show_verification_ui':
        await router.push('/verify');
        break;
      default:
        console.warn('Unknown continue_with action:', continueWith.action);
        break;
    }
  }

  async function submitFlow(
    flowType: KratosFlowType,
    flowId: string,
    data: unknown,
  ): Promise<KratosFlow | undefined> {
    try {
      if (flowType === 'recovery') {
        const result = await kratos.updateRecoveryFlow({
          flow: flowId,
          updateRecoveryFlowBody: data as UpdateRecoveryFlowBody,
        });
        return result.data;
      } else if (flowType === 'login') {
        const result = await kratos.updateLoginFlow({
          flow: flowId,
          updateLoginFlowBody: data as UpdateLoginFlowBody,
        });
        await reloadKratosSession();
        if (result.data.continue_with?.[0]) {
          await handleContinueWith(result.data.continue_with[0]);
        }
        return;
      } else if (flowType === 'registration') {
        const result = await kratos.updateRegistrationFlow({
          flow: flowId,
          updateRegistrationFlowBody: data as UpdateRegistrationFlowBody,
        });
        await reloadKratosSession();
        if (result.data.continue_with?.[0]) {
          await handleContinueWith(result.data.continue_with[0]);
        }
        return;
      } else if (flowType === 'verification') {
        const result = await kratos.updateVerificationFlow({
          flow: flowId,
          updateVerificationFlowBody: data as UpdateVerificationFlowBody,
        });
        return result.data;
      } else {
        const result = await kratos.updateSettingsFlow({
          flow: flowId,
          updateSettingsFlowBody: data as UpdateSettingsFlowBody,
        });
        return result.data;
      }
    } catch (error: unknown) {
      const response = error as { response?: { status: number; data: KratosFlow } };
      if (response.response) {
        return response.response.data;
      } else {
        console.error('Unexpected error during authentication:', error);
      }
    }
  }

  return { flow, submitFlow };
}

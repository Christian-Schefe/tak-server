import {
  computed,
  inject,
  ref,
  toValue,
  watch,
  type InjectionKey,
  type MaybeRefOrGetter,
  type Ref,
} from 'vue';

export interface FormContext {
  data: Record<string, unknown>;
  errors: Record<string, string>;
  reset: () => void;
}

export type FormValidatorResult<T> =
  | { type: 'success'; data: T }
  | { type: 'error'; errors: Record<string, string> };

export type FormValidator<T> = (data: Record<string, unknown>) => FormValidatorResult<T>;

export const FormKey: InjectionKey<Ref<FormContext>> = Symbol('FormContext');

export function createFormContext(
  initialData: MaybeRefOrGetter<Record<string, unknown> | undefined>,
) {
  function reset() {
    ctx.value.data = { ...toValue(initialData) };
  }
  const ctx = ref<FormContext>({
    data: { ...toValue(initialData) },
    errors: {},
    reset,
  });
  return ctx;
}

export function useFormValue(value: Ref<unknown>, name: MaybeRefOrGetter<string | undefined>) {
  const ctx = inject(FormKey, undefined);
  const formValue = computed(() => {
    const nameValue = toValue(name);
    if (!ctx || nameValue === undefined || !(nameValue in ctx.value.data)) {
      return undefined;
    }
    return { value: ctx.value.data[nameValue] };
  });
  watch(
    () => toValue(value),
    (newValue) => {
      const nameValue = toValue(name);
      if (!ctx || nameValue === undefined) {
        return undefined;
      }
      ctx.value.data[nameValue] = newValue;
    },
  );
  watch(
    formValue,
    (newFormValue) => {
      if (newFormValue !== undefined && newFormValue.value !== value.value) {
        value.value = newFormValue.value;
      }
    },
    { immediate: true },
  );
}

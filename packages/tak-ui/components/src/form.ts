import {
  computed,
  inject,
  provide,
  ref,
  toValue,
  watch,
  type InjectionKey,
  type MaybeRefOrGetter,
  type Ref,
} from 'vue';

export interface FormContext {
  data: Record<string, unknown>;
  errors: Record<string, string | undefined>;
}

export type FormValidator<T> = (
  data: Record<string, unknown>,
) => { type: 'success'; data: T } | { type: 'error'; errors: Record<string, string> };

export const FormKey: InjectionKey<Ref<FormContext>> = Symbol('FormContext');

export function provideFormContext(
  initialData: MaybeRefOrGetter<Record<string, unknown> | undefined>,
) {
  const ctx = ref<FormContext>({ data: { ...toValue(initialData) }, errors: {} });
  function resetForm() {
    ctx.value = { data: { ...toValue(initialData) }, errors: {} };
  }
  provide(FormKey, ctx);
  return { ctx, resetForm };
}

export function useFormValue(value: Ref<unknown>, name: MaybeRefOrGetter<string | undefined>) {
  const ctx = inject(FormKey, undefined);
  const formValue = computed(() => {
    const nameValue = toValue(name);
    if (!ctx || nameValue === undefined) {
      return undefined;
    }
    return ctx.value.data[nameValue];
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
      if (value.value === newFormValue) {
        return;
      }
      value.value = newFormValue;
    },
    { immediate: true },
  );
  return value;
}

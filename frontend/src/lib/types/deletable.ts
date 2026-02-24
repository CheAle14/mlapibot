export const SYM_DELETE = import.meta.env.DEV ? "__deleted" : Symbol();

export function stingifyDropDelete(object: unknown) {
  return JSON.stringify(
    object,
    function (this: unknown, key: string, value: any) {
      if (Array.isArray(value)) {
        return value.filter((v) => !isDeleted(v));
      } else if (isDeleted(value)) {
        return undefined;
      }

      return value;
    },
  );
}

export interface Deletable {
  [SYM_DELETE]?: true;
}

export function isDeleted(value: unknown): boolean {
  return (
    value !== null &&
    typeof value === "object" &&
    SYM_DELETE in value &&
    value[SYM_DELETE] === true
  );
}

import Root from "./alert.svelte";
import Description from "./alert-description.svelte";
import Title from "./alert-title.svelte";
import Error from "./alert-error.svelte";
export { alertVariants, type AlertVariant } from "./alert.svelte";

export {
  Root,
  Description,
  Title,
  Error,
  //
  Root as Alert,
  Description as AlertDescription,
  Title as AlertTitle,
  Error as AlertError,
};

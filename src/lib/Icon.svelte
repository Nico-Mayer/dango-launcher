<script lang="ts" module>
  import AppWindow from "@lucide/svelte/icons/app-window";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CirclePower from "@lucide/svelte/icons/circle-power";
  import ClipboardList from "@lucide/svelte/icons/clipboard-list";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Lock from "@lucide/svelte/icons/lock";
  import Moon from "@lucide/svelte/icons/moon";
  import Terminal from "@lucide/svelte/icons/terminal";
  import Trash2 from "@lucide/svelte/icons/trash-2";

  /// Every named icon the product can ask for. Icons are imported one by one
  /// rather than as a barrel so the bundle carries only what is used, and
  /// adding one is a deliberate line here rather than an arbitrary string
  /// somewhere in Rust.
  const REGISTRY = {
    "app-window": AppWindow,
    "circle-alert": CircleAlert,
    "circle-power": CirclePower,
    "clipboard-list": ClipboardList,
    "loader-circle": LoaderCircle,
    lock: Lock,
    moon: Moon,
    terminal: Terminal,
    "trash-2": Trash2,
  } as const;

  export type IconName = keyof typeof REGISTRY;

  /// An extension names an icon as `icon:<name>`; anything else is a path to an
  /// image file it extracted, such as an application's own icon.
  const NAMED_PREFIX = "icon:";

  export function namedIcon(value: string | null | undefined): IconName | null {
    if (!value?.startsWith(NAMED_PREFIX)) return null;
    const name = value.slice(NAMED_PREFIX.length);
    return name in REGISTRY ? (name as IconName) : null;
  }

  export function isNamedIcon(value: string | null | undefined): boolean {
    return !!value?.startsWith(NAMED_PREFIX);
  }
</script>

<script lang="ts">
  interface Props {
    name: IconName;
    size?: number;
    class?: string;
  }

  let { name, size = 20, class: className = "" }: Props = $props();

  const Component = $derived(REGISTRY[name]);
</script>

<Component {size} class={className} absoluteStrokeWidth />

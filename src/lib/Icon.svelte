<script lang="ts" module>
  import AppWindow from "@lucide/svelte/icons/app-window";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CirclePower from "@lucide/svelte/icons/circle-power";
  import ClipboardList from "@lucide/svelte/icons/clipboard-list";
  import ClipboardType from "@lucide/svelte/icons/clipboard-type";
  import Columns3 from "@lucide/svelte/icons/columns-3";
  import Grid2x2 from "@lucide/svelte/icons/grid-2x2";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Lock from "@lucide/svelte/icons/lock";
  import Link from "@lucide/svelte/icons/link";
  import Maximize from "@lucide/svelte/icons/maximize";
  import Maximize2 from "@lucide/svelte/icons/maximize-2";
  import Minimize2 from "@lucide/svelte/icons/minimize-2";
  import Monitor from "@lucide/svelte/icons/monitor";
  import Moon from "@lucide/svelte/icons/moon";
  import PanelBottom from "@lucide/svelte/icons/panel-bottom";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import PanelRight from "@lucide/svelte/icons/panel-right";
  import PanelTop from "@lucide/svelte/icons/panel-top";
  import Square from "@lucide/svelte/icons/square";
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
    "clipboard-type": ClipboardType,
    "columns-3": Columns3,
    "grid-2x2": Grid2x2,
    "loader-circle": LoaderCircle,
    link: Link,
    lock: Lock,
    maximize: Maximize,
    "maximize-2": Maximize2,
    "minimize-2": Minimize2,
    monitor: Monitor,
    moon: Moon,
    "panel-bottom": PanelBottom,
    "panel-left": PanelLeft,
    "panel-right": PanelRight,
    "panel-top": PanelTop,
    square: Square,
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

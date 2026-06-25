<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  type ResizeDir =
    | 'North'
    | 'South'
    | 'East'
    | 'West'
    | 'NorthEast'
    | 'NorthWest'
    | 'SouthEast'
    | 'SouthWest';

  const handles: { cls: string; dir: ResizeDir }[] = [
    { cls: 'n', dir: 'North' },
    { cls: 's', dir: 'South' },
    { cls: 'e', dir: 'East' },
    { cls: 'w', dir: 'West' },
    { cls: 'ne', dir: 'NorthEast' },
    { cls: 'nw', dir: 'NorthWest' },
    { cls: 'se', dir: 'SouthEast' },
    { cls: 'sw', dir: 'SouthWest' }
  ];

  function start(dir: ResizeDir, e: MouseEvent) {
    if (e.button !== 0) return;
    getCurrentWindow().startResizeDragging(dir);
  }
</script>

{#each handles as h (h.cls)}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="rz {h.cls}" onmousedown={(e) => start(h.dir, e)}></div>
{/each}

<style>
  .rz {
    position: fixed;
    z-index: 100;
  }
  .n {
    top: 0;
    left: 9px;
    right: 9px;
    height: 4px;
    cursor: ns-resize;
  }
  .s {
    bottom: 0;
    left: 9px;
    right: 9px;
    height: 4px;
    cursor: ns-resize;
  }
  .e {
    top: 9px;
    bottom: 9px;
    right: 0;
    width: 4px;
    cursor: ew-resize;
  }
  .w {
    top: 9px;
    bottom: 9px;
    left: 0;
    width: 4px;
    cursor: ew-resize;
  }
  .ne {
    top: 0;
    right: 0;
    width: 9px;
    height: 9px;
    cursor: nesw-resize;
  }
  .nw {
    top: 0;
    left: 0;
    width: 9px;
    height: 9px;
    cursor: nwse-resize;
  }
  .se {
    bottom: 0;
    right: 0;
    width: 9px;
    height: 9px;
    cursor: nwse-resize;
  }
  .sw {
    bottom: 0;
    left: 0;
    width: 9px;
    height: 9px;
    cursor: nesw-resize;
  }
</style>

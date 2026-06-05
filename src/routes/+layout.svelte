<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  
  let { children } = $props();

  onMount(() => {
    // Prevent default web browser context menu on layout elements, 
    // but keep it active on input fields for easy copying/pasting.
    const handleContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
      if (!isInput) {
        e.preventDefault();
      }
    };

    document.addEventListener('contextmenu', handleContextMenu);
    return () => {
      document.removeEventListener('contextmenu', handleContextMenu);
    };
  });
</script>

{@render children()}

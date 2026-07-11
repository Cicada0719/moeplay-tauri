<script lang="ts">
  import { getThemePack, normalizeAppearance } from "../theme-packs";
  import { fileSrc } from "../utils";
  import { settingsStore } from "../stores/settings.svelte";
  import { wallpaperStore } from "../stores/wallpapers.svelte";

  let { surface = "browse" }: { surface?: "browse" | "management" | "immersive" } = $props();
  let ready = $state(false);
  const appearance = $derived(normalizeAppearance(settingsStore.settings.appearance));
  const pack = $derived(getThemePack(appearance.theme_pack));
  const remoteWallpapers = $derived(wallpaperStore.installedFor(pack.id, settingsStore.settings.nsfw_display_mode ?? "blur").map((record) => ({
    id: record.asset.id,
    title: record.asset.title,
    src: fileSrc(record.local_path) ?? pack.wallpapers[0].src,
    placeholder: pack.wallpapers[0].placeholder,
    rating: record.asset.rating,
  })));
  const wallpapers = $derived([...pack.wallpapers, ...remoteWallpapers]);
  const selected = $derived.by(() => {
    if (appearance.custom_wallpaper_path) return { id: "custom", title: "自定义壁纸", src: fileSrc(appearance.custom_wallpaper_path) ?? pack.wallpapers[0].src, placeholder: pack.wallpapers[0].placeholder, rating: "general" as const };
    if (appearance.wallpaper_rotation === "fixed" && appearance.fixed_wallpaper_id) return wallpapers.find((item) => item.id === appearance.fixed_wallpaper_id) ?? pack.wallpapers[0];
    let index = 0;
    try {
      const seedKey = `moeplay-wallpaper-session:${pack.id}`;
      const existing = sessionStorage.getItem(seedKey);
      if (existing !== null) index = Number(existing) || 0;
      else { index = Math.floor(Math.random() * wallpapers.length); sessionStorage.setItem(seedKey, String(index)); }
    } catch { index = new Date().getDate() % wallpapers.length; }
    return wallpapers[index % wallpapers.length];
  });
  const decoration = $derived(appearance.decorative_effects && appearance.color_mode !== "contrast" ? pack.decoration : null);
  const particles = Array.from({ length: 18 }, (_, index) => ({ left: (index * 47) % 100, delay: (index * .63) % 8, duration: 5 + (index % 7) }));
  $effect(() => {
    selected.src; ready = false;
    if (typeof document !== "undefined") document.documentElement.dataset.wallpaperRating = selected.rating;
  });
</script>

<div class="wallpaper-stage" data-surface={surface} data-wallpaper-id={selected.id} aria-hidden="true">
  <img class="wallpaper-stage__placeholder" src={selected.placeholder} alt="" />
  <img class="wallpaper-stage__image" class:is-ready={ready} src={selected.src} alt="" onload={() => (ready = true)} />
  <div class="wallpaper-stage__ambient"></div>
  <div class="wallpaper-stage__scrim"></div>
  {#if decoration}
    <div class="wallpaper-stage__decor wallpaper-stage__decor--{decoration}">
      {#each particles as particle}
        <span style={`left:${particle.left}%;animation-delay:${particle.delay}s;animation-duration:${particle.duration}s`}></span>
      {/each}
    </div>
  {/if}
</div>

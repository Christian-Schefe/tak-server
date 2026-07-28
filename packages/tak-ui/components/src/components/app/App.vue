<template>
  <div class="p-app">
    <slot name="top" />
    <div class="p-app-horizontal">
      <slot name="left" />
      <div class="p-app-content"><slot /></div>
      <slot name="right" />
    </div>
    <slot name="bottom" />
  </div>
</template>
<style lang="css" scoped>
.p-app {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
}
.p-app-horizontal {
  display: flex;
  flex-direction: row;
  height: 0;
  flex-grow: 1;
  overflow: hidden;
  position: relative;
}
.p-app-content {
  width: 0;
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  overflow: auto;
  scrollbar-gutter: stable both-edges;
}
</style>

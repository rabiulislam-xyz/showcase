<div class="skeleton-card" aria-hidden="true">
  <div class="skel skel-icon"></div>
  <div class="skel skel-line"></div>
  <div class="skel skel-line short"></div>
  <div class="skel skel-line tag"></div>
</div>

<style>
  .skeleton-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    padding: 18px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .skel {
    background: linear-gradient(
      90deg,
      var(--surface-2) 0%,
      var(--border) 50%,
      var(--surface-2) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
    border-radius: 6px;
  }
  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
  .skel-icon {
    width: 56px;
    height: 56px;
    border-radius: 14px;
  }
  .skel-line {
    height: 12px;
  }
  .skel-line.short {
    width: 60%;
  }
  .skel-line.tag {
    width: 40%;
    height: 16px;
    border-radius: 5px;
    margin-top: auto;
  }
</style>

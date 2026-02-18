import { Component } from "@angular/core";

@Component({
    selector: "app-icon-compass-logo",
    standalone: true,
    styles: `
        :host {
            display: block;
            width: 100%;
            height: 100%;
        }
        svg {
            width: 100%;
            height: 100%;
            display: block;
        }
        .compass-ring {
            fill: none;
            stroke: var(--borders-default);
            stroke-width: 3;
        }
        .compass-ring.inner {
            stroke: rgba(96, 90, 82, 0.15);
            stroke-dasharray: 4 6;
        }
        .compass-core {
            fill: var(--text-subtle);
        }
    `,
    template: `
        <svg viewBox="0 0 200 200" role="presentation" focusable="false">
            <circle class="compass-ring outer" cx="100" cy="100" r="70" />
            <circle class="compass-ring inner" cx="100" cy="100" r="48" />
            <line class="compass-axis" x1="100" y1="20" x2="100" y2="180" />
            <line class="compass-axis" x1="20" y1="100" x2="180" y2="100" />
            <path
                class="compass-needle"
                d="M100 40 L124 100 L100 160 L76 100 Z"
                transform="rotate(45 100 100)"
            />
            <circle class="compass-core" cx="100" cy="100" r="6" />
        </svg>
    `,
})
export class IconCompassLogoComponent {}

import { Component } from "@angular/core";

@Component({
    selector: "app-icon-collapse-chevrons",
    standalone: true,
    styles: `
        :host {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            line-height: 0;
        }
        svg {
            width: 18px;
            height: 30px;
            display: block;
        }
    `,
    template: `
        <svg
            class="diff-thread-collapse-glyph"
            viewBox="0 0 12 16"
            aria-hidden="true"
        >
            <polyline
                points="3,2 6,6 9,2"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
            />
            <polyline
                points="3,14 6,10 9,14"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
            />
        </svg>
    `,
})
export class IconCollapseChevronsComponent {}

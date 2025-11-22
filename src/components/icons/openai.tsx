import type { SVGProps } from "react";
import { useId } from "react";

export const OpenAIIcon = (props: SVGProps<SVGSVGElement>) => {
	const gradientId = useId();
	const bgId = `${gradientId}-openai-bg`;
	const strokeId = `${gradientId}-openai-stroke`;

	return (
		<svg
			viewBox="0 0 64 64"
			role="img"
			aria-label="OpenAI"
			xmlns="http://www.w3.org/2000/svg"
			{...props}
		>
			<defs>
				<linearGradient
					id={bgId}
					x1="12"
					y1="52"
					x2="52"
					y2="12"
					gradientUnits="userSpaceOnUse"
				>
					<stop stopColor="#101318" />
					<stop offset="1" stopColor="#1E2230" />
				</linearGradient>
				<linearGradient
					id={strokeId}
					x1="20"
					y1="44"
					x2="44"
					y2="20"
					gradientUnits="userSpaceOnUse"
				>
					<stop stopColor="#ffffff" stopOpacity="0.9" />
					<stop offset="1" stopColor="#9fb6ff" />
				</linearGradient>
			</defs>
			<rect width="64" height="64" rx="16" fill={`url(#${bgId})`} />
			<g fill="none" stroke={`url(#${strokeId})`} strokeWidth="4">
				<path
					d="M42 34c2.5-1.7 4.1-4.7 4.1-7.7 0-6-4.8-10.9-10.9-10.9-2.8 0-5.4 1.1-7.4 2.9l3.2 3.4c1.1-1.1 2.6-1.7 4.2-1.7 4.1 0 7.3 3.3 7.3 7.3 0 1.8-.7 3.6-1.9 4.8z"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
				<path
					d="M22 30c-2.5 1.7-4.1 4.7-4.1 7.7 0 6 4.8 10.9 10.9 10.9 2.8 0 5.4-1.1 7.4-2.9l-3.2-3.4c-1.1 1.1-2.6 1.7-4.2 1.7-4.1 0-7.3-3.3-7.3-7.3 0-1.8.7-3.6 1.9-4.8z"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
				<path
					d="M24.5 21.5c-3.6 1.4-6.2 5-6.2 9.2 0 3.2 1.2 5.9 3.4 7.9l3-3.5c-1.2-1.1-1.9-2.7-1.9-4.4 0-2.6 1.5-4.9 3.8-5.9z"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
				<path
					d="M39.5 42.5c3.6-1.4 6.2-5 6.2-9.2 0-3.2-1.2-5.9-3.4-7.9l-3 3.5c1.2 1.1 1.9 2.7 1.9 4.4 0 2.6-1.5 4.9-3.8 5.9z"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
				<path
					d="M28 22c-2.6-2.1-5.9-2.6-7.8-2.6M36 42c2.6 2.1 5.9 2.6 7.8 2.6"
					strokeLinecap="round"
				/>
			</g>
		</svg>
	);
};

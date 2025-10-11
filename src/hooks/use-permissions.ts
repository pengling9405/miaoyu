import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type OSPermission } from "~/lib/tauri";

export function useCheckPermissions(initialCheck: boolean = false) {
	return useQuery({
		queryKey: ["permissions", initialCheck],
		queryFn: async () => {
			return await commands.checkOsPermissions(initialCheck);
		},
	});
}

export function useRequestPermission() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (permission: OSPermission) => {
			await commands.requestPermission(permission);
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["permissions"] });
		},
		onError: (error) => {
			console.error("Failed to request permission:", error);
		},
	});
}

export function useOpenPermissionSettings() {
	return useMutation({
		mutationFn: async (permission: OSPermission) => {
			await commands.openPermissionSettings(permission);
		},
		onError: (error) => {
			console.error("Failed to open permission settings:", error);
		},
	});
}

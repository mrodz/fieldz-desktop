import { goto } from '$app/navigation';

export function load({ params }) {
	const fieldId = Number(params.fieldId);

	if (isNaN(fieldId)) {
		alert(`Field ID is not a number: ${params.fieldId}`);
		goto('/');
	}

	return {
		fieldId
	};
}

const keyForm = document.getElementById('key-form');
const keyInput = document.getElementById('key');
const errorMessage = document.getElementById('error-message');

keyForm.addEventListener('submit', (event) => {
  event.preventDefault();

  const key = keyInput.value.trim();

  if (key === '') {
    errorMessage.textContent = 'Please enter the key.';
    errorMessage.style.display = 'block';
    return;
  }

  // Replace this with your logic to handle the key (e.g., redirect to the game)
  console.log('Submitted key:', key);
  // You can redirect to the game or display a success message here
});
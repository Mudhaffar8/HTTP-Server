/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{html,js}"],
  theme: {
    extend: {
      fontFamily: {
        primary: "var(--ff-primary)",
        secondary: "var(--ff-secondary)",
      },
      colors: {
        primary: {
          one: "var(--c-primary-one)",
          two: "var(--c-primary-two)",
        },
        neutral : {
          one: "var(--c-neutral-one)",
          two: "var(--c-neutral-two)",
        }
      }
    },
  },
  plugins: [],
}

